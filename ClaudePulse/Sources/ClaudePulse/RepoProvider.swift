@preconcurrency import Foundation
import SwiftUI

// MARK: - Models

struct RepoSession: Identifiable, Sendable {
    let sessionId: String
    let lastModified: Date
    let gitBranch: String

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

    var sessionCount: Int { sessions.count }
    var id: String { projectPath }
}

// MARK: - Provider

@MainActor
final class RepoProvider: ObservableObject {
    @Published var repos: [RepoInfo] = []

    nonisolated private static let claudeProjectsPath = AppConfig.projectsPath

    func refresh() async {
        let results = await Task.detached {
            Self.scanRepos()
        }.value
        self.repos = results
    }

    private nonisolated static func scanRepos() -> [RepoInfo] {
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

            // Extract project info from first file
            let (projectPath, projectName) = extractProjectInfo(dirPath: dirPath, fileName: jsonlFiles[0])

            // Build session list
            var sessions: [RepoSession] = []
            for file in jsonlFiles {
                let filePath = "\(dirPath)/\(file)"
                let sessionId = (file as NSString).deletingPathExtension
                let attrs = try? fm.attributesOfItem(atPath: filePath)
                let mtime = attrs?[.modificationDate] as? Date ?? Date.distantPast
                let branch = extractGitBranch(filePath: filePath) ?? "unknown"
                sessions.append(RepoSession(sessionId: sessionId, lastModified: mtime, gitBranch: branch))
            }

            sessions.sort { $0.lastModified > $1.lastModified }

            repos.append(RepoInfo(
                projectPath: projectPath ?? dirPath,
                projectName: projectName ?? dir,
                sessions: sessions
            ))
        }

        repos.sort { $0.sessionCount > $1.sessionCount }
        return repos
    }

    /// Reads the first user message from a JSONL file to extract `cwd` and derive the project name.
    nonisolated static func extractProjectInfo(dirPath: String, fileName: String) -> (String?, String?) {
        let filePath = "\(dirPath)/\(fileName)"
        guard let handle = FileHandle(forReadingAtPath: filePath) else { return (nil, nil) }
        defer { handle.closeFile() }

        let chunk = handle.readData(ofLength: 8192)
        guard let content = String(data: chunk, encoding: .utf8) else { return (nil, nil) }

        for line in content.components(separatedBy: "\n") {
            guard !line.isEmpty,
                  let data = line.data(using: .utf8),
                  let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                  json["type"] as? String == "user",
                  let cwd = json["cwd"] as? String
            else { continue }

            let components = cwd.components(separatedBy: "/").filter { !$0.isEmpty }
            let name: String
            if components.count >= 2 {
                name = "\(components[components.count - 2])/\(components[components.count - 1])"
            } else {
                name = components.last ?? cwd
            }

            return (cwd, name)
        }

        return (nil, nil)
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
