@preconcurrency import Foundation
import SwiftUI

// MARK: - Models

struct RepoSession: Identifiable, Sendable {
    let sessionId: String
    let lastModified: Date
    let gitBranch: String
    let worktreeLabel: String?

    var id: String { sessionId }

    /// Formats lastModified as a relative time string.
    var timeAgo: String {
        let elapsed = Int(Date().timeIntervalSince(lastModified))
        if elapsed < 60 { return "<1m ago" }
        let minutes = elapsed / 60
        if minutes < 60 { return "\(minutes)m ago" }
        let hours = minutes / 60
        if hours < 24 { return "\(hours)h ago" }
        let days = hours / 24
        if days < 30 { return "\(days)d ago" }
        return "\(days / 30)mo ago"
    }

    /// First 8 characters of session ID.
    var shortId: String {
        String(sessionId.prefix(8))
    }
}

struct RepoInfo: Identifiable, Sendable {
    let projectPath: String
    let projectName: String
    let sessions: [RepoSession]
    let worktreePaths: Set<String>

    var sessionCount: Int { sessions.count }
    var worktreeCount: Int { worktreePaths.count }
    var id: String { projectPath }
}

// MARK: - Provider

@MainActor
final class RepoProvider: ObservableObject {
    @Published var repos: [RepoInfo] = []

    nonisolated private static let claudeProjectsPath = AppConfig.projectsPath

    func refresh(enrichments: [String: SessionEnrichment] = [:]) async {
        let enrichmentsCopy = enrichments
        let results = await Task.detached {
            Self.scanRepos(enrichments: enrichmentsCopy)
        }.value
        self.repos = results
    }

    private nonisolated static func scanRepos(enrichments: [String: SessionEnrichment]) -> [RepoInfo] {
        let fm = FileManager.default
        let basePath = claudeProjectsPath

        guard let dirs = try? fm.contentsOfDirectory(atPath: basePath) else { return [] }

        var repos: [RepoInfo] = []

        for dir in dirs {
            let dirPath = "\(basePath)/\(dir)"
            var isDir: ObjCBool = false
            guard fm.fileExists(atPath: dirPath, isDirectory: &isDir), isDir.boolValue else { continue }

            guard let files = try? fm.contentsOfDirectory(atPath: dirPath) else { continue }
            let jsonlFiles = files.filter { $0.hasSuffix(".jsonl") }
            guard !jsonlFiles.isEmpty else { continue }

            // Extract project info from first file (includes cwd for worktree heuristic)
            let (projectPath, projectName, worktreeRoot) = extractProjectInfoWithWorktree(dirPath: dirPath, fileName: jsonlFiles[0])

            // Build session list
            var sessions: [RepoSession] = []
            var worktreePaths: Set<String> = []
            for file in jsonlFiles {
                let filePath = "\(dirPath)/\(file)"
                let sessionId = (file as NSString).deletingPathExtension
                let attrs = try? fm.attributesOfItem(atPath: filePath)
                let mtime = attrs?[.modificationDate] as? Date ?? Date.distantPast
                let branch = extractGitBranch(filePath: filePath) ?? "unknown"

                // Determine worktree label from enrichment or heuristic
                let wtLabel: String?
                if let enrichment = enrichments[sessionId], let label = enrichment.worktreeLabel {
                    wtLabel = label
                } else if let cwd = extractCwd(filePath: filePath) {
                    wtLabel = detectWorktreeLabelFromCwd(cwd)
                } else {
                    wtLabel = nil
                }

                if let label = wtLabel {
                    worktreePaths.insert(label)
                }

                sessions.append(RepoSession(sessionId: sessionId, lastModified: mtime, gitBranch: branch, worktreeLabel: wtLabel))
            }

            sessions.sort { $0.lastModified > $1.lastModified }

            let effectivePath: String
            let effectiveName: String
            if let root = worktreeRoot {
                effectivePath = root
                effectiveName = projectNameFromPath(root)
            } else {
                effectivePath = projectPath ?? dirPath
                effectiveName = projectName ?? dir
            }

            repos.append(RepoInfo(
                projectPath: effectivePath,
                projectName: effectiveName,
                sessions: sessions,
                worktreePaths: worktreePaths
            ))
        }

        // Merge repos that share the same canonical root
        repos = mergeWorktreeRepos(repos, enrichments: enrichments)

        repos.sort { $0.sessionCount > $1.sessionCount }
        return repos
    }

    // MARK: - Worktree Grouping

    /// Merges repos that share the same canonical repo root (from heuristic or enrichments).
    private nonisolated static func mergeWorktreeRepos(_ repos: [RepoInfo], enrichments: [String: SessionEnrichment]) -> [RepoInfo] {
        // Build a mapping: canonical root → [RepoInfo indices]
        var rootToIndices: [String: [Int]] = [:]
        for (i, repo) in repos.enumerated() {
            // Check enrichments for canonical root
            var canonicalRoot: String?
            for session in repo.sessions {
                if let enrichment = enrichments[session.sessionId], let root = enrichment.canonicalRepoRoot {
                    canonicalRoot = root
                    break
                }
            }

            let key = canonicalRoot ?? repo.projectPath
            rootToIndices[key, default: []].append(i)
        }

        var merged: [RepoInfo] = []
        var consumed: Set<Int> = []

        for (root, indices) in rootToIndices {
            guard !indices.isEmpty else { continue }
            for idx in indices { consumed.insert(idx) }

            if indices.count == 1 {
                merged.append(repos[indices[0]])
                continue
            }

            // Merge multiple repos under the same root
            var allSessions: [RepoSession] = []
            var allWorktreePaths: Set<String> = []
            for idx in indices {
                allSessions.append(contentsOf: repos[idx].sessions)
                allWorktreePaths.formUnion(repos[idx].worktreePaths)
                // Also add the original project path as a worktree path if it differs from root
                if repos[idx].projectPath != root {
                    let label = (repos[idx].projectPath as NSString).lastPathComponent
                    allWorktreePaths.insert(label)
                }
            }
            allSessions.sort { $0.lastModified > $1.lastModified }

            merged.append(RepoInfo(
                projectPath: root,
                projectName: projectNameFromPath(root),
                sessions: allSessions,
                worktreePaths: allWorktreePaths
            ))
        }

        return merged
    }

    // MARK: - Helpers

    /// Extract last 2 path components as a project name.
    nonisolated static func projectNameFromPath(_ path: String) -> String {
        let components = path.components(separatedBy: "/").filter { !$0.isEmpty }
        if components.count >= 2 {
            return "\(components[components.count - 2])/\(components[components.count - 1])"
        }
        return components.last ?? path
    }

    /// Detects `.claude/worktrees/<name>` in a cwd and returns the worktree label.
    private nonisolated static func detectWorktreeLabelFromCwd(_ cwd: String) -> String? {
        let marker = "/.claude/worktrees/"
        guard let idx = cwd.range(of: marker) else { return nil }
        let after = String(cwd[idx.upperBound...])
        let label = after.components(separatedBy: "/").first ?? ""
        return label.isEmpty ? nil : label
    }

    /// Detects `.claude/worktrees/` in a cwd and returns the repo root.
    private nonisolated static func detectWorktreeRootFromCwd(_ cwd: String) -> String? {
        let marker = "/.claude/worktrees/"
        guard let range = cwd.range(of: marker) else { return nil }
        let root = String(cwd[..<range.lowerBound])
        return root.isEmpty ? nil : root
    }

    /// Reads the first user message from a JSONL file to extract `cwd`, project name, and worktree root.
    nonisolated static func extractProjectInfoWithWorktree(dirPath: String, fileName: String) -> (String?, String?, String?) {
        let filePath = "\(dirPath)/\(fileName)"
        guard let handle = FileHandle(forReadingAtPath: filePath) else { return (nil, nil, nil) }
        defer { handle.closeFile() }

        let chunk = handle.readData(ofLength: 8192)
        guard let content = String(data: chunk, encoding: .utf8) else { return (nil, nil, nil) }

        for line in content.components(separatedBy: "\n") {
            guard !line.isEmpty,
                  let data = line.data(using: .utf8),
                  let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                  json["type"] as? String == "user",
                  let cwd = json["cwd"] as? String
            else { continue }

            let worktreeRoot = detectWorktreeRootFromCwd(cwd)
            let effectivePath = worktreeRoot ?? cwd
            let name = projectNameFromPath(effectivePath)

            return (cwd, name, worktreeRoot)
        }

        return (nil, nil, nil)
    }

    /// Reads the cwd from the first user message in a JSONL file.
    private nonisolated static func extractCwd(filePath: String) -> String? {
        guard let handle = FileHandle(forReadingAtPath: filePath) else { return nil }
        defer { handle.closeFile() }

        let chunk = handle.readData(ofLength: 4096)
        guard let content = String(data: chunk, encoding: .utf8) else { return nil }

        for line in content.components(separatedBy: "\n") {
            guard !line.isEmpty,
                  let data = line.data(using: .utf8),
                  let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                  json["type"] as? String == "user",
                  let cwd = json["cwd"] as? String
            else { continue }
            return cwd
        }

        return nil
    }

    /// Reads the first user message from a JSONL file to extract `gitBranch`.
    nonisolated static func extractGitBranch(filePath: String) -> String? {
        guard let handle = FileHandle(forReadingAtPath: filePath) else { return nil }
        defer { handle.closeFile() }

        let chunk = handle.readData(ofLength: 4096)
        guard let content = String(data: chunk, encoding: .utf8) else { return nil }

        for line in content.components(separatedBy: "\n") {
            guard !line.isEmpty,
                  let data = line.data(using: .utf8),
                  let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                  json["type"] as? String == "user",
                  let branch = json["gitBranch"] as? String
            else { continue }
            return branch
        }

        return nil
    }
}
