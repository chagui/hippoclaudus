@preconcurrency import Foundation
import SwiftUI

// MARK: - Executable Finder

enum ExecutableFinder {
    static let gitPath: String? = resolve("git")
    static let ghPath: String? = resolve("gh")

    private static let searchPaths = [
        "/opt/homebrew/bin",
        "/usr/local/bin",
        "/usr/bin",
    ]

    private static func resolve(_ name: String) -> String? {
        let fm = FileManager.default
        for dir in searchPaths {
            let path = "\(dir)/\(name)"
            if fm.isExecutableFile(atPath: path) {
                return path
            }
        }
        return nil
    }
}

// MARK: - Models

struct PRInfo: Sendable {
    let number: Int
    let title: String
    let url: String
    let state: String   // "OPEN", "MERGED", "CLOSED"
    let isDraft: Bool
}

struct SessionEnrichment: Sendable {
    let isGitRepo: Bool
    let isWorktree: Bool
    let prInfo: PRInfo?
}

// MARK: - Provider

@MainActor
final class GitEnrichmentProvider: ObservableObject {
    @Published var enrichments: [String: SessionEnrichment] = [:]

    func enrich(sessions: [ActiveSessionResponse]) async {
        let results = await withTaskGroup(
            of: (String, SessionEnrichment).self,
            returning: [String: SessionEnrichment].self
        ) { group in
            for session in sessions {
                group.addTask {
                    await self.enrichSession(session)
                }
            }

            var dict: [String: SessionEnrichment] = [:]
            for await (id, enrichment) in group {
                dict[id] = enrichment
            }
            return dict
        }

        self.enrichments = results
    }

    private nonisolated func enrichSession(_ session: ActiveSessionResponse) async -> (String, SessionEnrichment) {
        let cwd = session.projectCwd

        // Skip enrichment entirely if not inside a git repository
        guard await isGitRepo(cwd: cwd) else {
            return (session.sessionId, SessionEnrichment(isGitRepo: false, isWorktree: false, prInfo: nil))
        }

        async let worktree = detectWorktree(cwd: cwd)
        async let pr = fetchPR(cwd: cwd)

        let enrichment = await SessionEnrichment(isGitRepo: true, isWorktree: worktree, prInfo: pr)
        return (session.sessionId, enrichment)
    }

    // MARK: - Git Repo Check

    private nonisolated func isGitRepo(cwd: String) async -> Bool {
        guard let gitPath = ExecutableFinder.gitPath else { return false }

        guard let data = await runCommand(
            executablePath: gitPath,
            arguments: ["rev-parse", "--is-inside-work-tree"],
            workingDirectory: cwd,
            silent: true
        ) else {
            return false
        }

        let output = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines)
        return output == "true"
    }

    // MARK: - Worktree Detection

    private nonisolated func detectWorktree(cwd: String) async -> Bool {
        guard let gitPath = ExecutableFinder.gitPath else { return false }

        async let commonDir = runCommand(
            executablePath: gitPath,
            arguments: ["rev-parse", "--git-common-dir"],
            workingDirectory: cwd
        )
        async let gitDir = runCommand(
            executablePath: gitPath,
            arguments: ["rev-parse", "--git-dir"],
            workingDirectory: cwd
        )

        guard let commonData = await commonDir,
              let gitData = await gitDir,
              let common = String(data: commonData, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines),
              let git = String(data: gitData, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines)
        else {
            return false
        }

        // Resolve relative paths against cwd
        let cwdURL = URL(fileURLWithPath: cwd)
        let resolvedCommon = URL(fileURLWithPath: common, relativeTo: cwdURL).standardized.path
        let resolvedGit = URL(fileURLWithPath: git, relativeTo: cwdURL).standardized.path

        return resolvedCommon != resolvedGit
    }

    // MARK: - PR Fetch

    private nonisolated func fetchPR(cwd: String) async -> PRInfo? {
        guard let ghPath = ExecutableFinder.ghPath else { return nil }

        guard let data = await runCommand(
            executablePath: ghPath,
            arguments: ["pr", "view", "--json", "number,title,url,state,isDraft"],
            workingDirectory: cwd,
            silent: true  // Expected to fail when no PR exists for the branch
        ) else {
            return nil
        }

        struct GHPRResponse: Decodable {
            let number: Int
            let title: String
            let url: String
            let state: String
            let isDraft: Bool
        }

        guard let decoded = try? JSONDecoder().decode(GHPRResponse.self, from: data) else {
            return nil
        }

        return PRInfo(
            number: decoded.number,
            title: decoded.title,
            url: decoded.url,
            state: decoded.state,
            isDraft: decoded.isDraft
        )
    }

    // MARK: - Command Runner

    private nonisolated func runCommand(
        executablePath: String,
        arguments: [String],
        workingDirectory: String,
        silent: Bool = false
    ) async -> Data? {
        await Task.detached {
            let process = Process()
            process.executableURL = URL(fileURLWithPath: executablePath)
            process.arguments = arguments
            process.currentDirectoryURL = URL(fileURLWithPath: workingDirectory)

            let stdout = Pipe()
            process.standardOutput = stdout
            process.standardError = Pipe()

            do {
                try process.run()
            } catch {
                if !silent {
                    NSLog("GitEnrichment: failed to run %@ %@: %@", executablePath, arguments.joined(separator: " "), error.localizedDescription)
                }
                return nil as Data?
            }

            process.waitUntilExit()

            guard process.terminationStatus == 0 else {
                if !silent {
                    NSLog("GitEnrichment: %@ exited with status %d in %@", executablePath, process.terminationStatus, workingDirectory)
                }
                return nil as Data?
            }

            return stdout.fileHandleForReading.readDataToEndOfFile()
        }.value
    }
}
