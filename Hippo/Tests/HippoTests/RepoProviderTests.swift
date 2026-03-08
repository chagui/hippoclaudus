import Foundation
@testable import Hippo
import Testing

struct RepoProviderTests {
    // MARK: - RepoSession.timeAgo

    @Test func timeAgoLessThanOneMinute() {
        let session = RepoSession(
            sessionId: "abc",
            lastModified: Date().addingTimeInterval(-30),
            gitBranch: "main",
            worktreeLabel: nil,
        )
        #expect(session.timeAgo == "<1m ago")
    }

    @Test func timeAgoMinutes() {
        let session = RepoSession(
            sessionId: "abc",
            lastModified: Date().addingTimeInterval(-300),
            gitBranch: "main",
            worktreeLabel: nil,
        )
        #expect(session.timeAgo == "5m ago")
    }

    @Test func timeAgoHours() {
        let session = RepoSession(
            sessionId: "abc",
            lastModified: Date().addingTimeInterval(-7200),
            gitBranch: "main",
            worktreeLabel: nil,
        )
        #expect(session.timeAgo == "2h ago")
    }

    @Test func timeAgoDays() {
        let session = RepoSession(
            sessionId: "abc",
            lastModified: Date().addingTimeInterval(-172_800),
            gitBranch: "main",
            worktreeLabel: nil,
        )
        #expect(session.timeAgo == "2d ago")
    }

    @Test func timeAgoMonths() {
        let session = RepoSession(
            sessionId: "abc",
            lastModified: Date().addingTimeInterval(-86400 * 45),
            gitBranch: "main",
            worktreeLabel: nil,
        )
        #expect(session.timeAgo == "1mo ago")
    }

    // MARK: - RepoSession.shortId

    @Test func shortId() {
        let session = RepoSession(
            sessionId: "abcdefghijklmnop",
            lastModified: Date(),
            gitBranch: "main",
            worktreeLabel: nil,
        )
        #expect(session.shortId == "abcdefgh")
    }

    @Test func shortIdShortSession() {
        let session = RepoSession(
            sessionId: "abc",
            lastModified: Date(),
            gitBranch: "main",
            worktreeLabel: nil,
        )
        #expect(session.shortId == "abc")
    }

    // MARK: - extractProjectInfoWithWorktree

    @Test func extractProjectInfoValid() throws {
        let dir = NSTemporaryDirectory() + UUID().uuidString
        try FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(atPath: dir) }

        let jsonl = #"{"type":"user","cwd":"/Users/dev/my-org/my-project","message":{"content":"hello"}}"# + "\n"
        let fileName = "session.jsonl"
        try jsonl.write(toFile: "\(dir)/\(fileName)", atomically: true, encoding: .utf8)

        let (path, name, worktreeRoot) = RepoProvider.extractProjectInfoWithWorktree(dirPath: dir, fileName: fileName)
        #expect(path == "/Users/dev/my-org/my-project")
        #expect(name == "my-org/my-project")
        #expect(worktreeRoot == nil)
    }

    @Test func extractProjectInfoMissingCwd() throws {
        let dir = NSTemporaryDirectory() + UUID().uuidString
        try FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(atPath: dir) }

        let jsonl = #"{"type":"user","message":{"content":"hello"}}"# + "\n"
        let fileName = "session.jsonl"
        try jsonl.write(toFile: "\(dir)/\(fileName)", atomically: true, encoding: .utf8)

        let (path, name, worktreeRoot) = RepoProvider.extractProjectInfoWithWorktree(dirPath: dir, fileName: fileName)
        #expect(path == nil)
        #expect(name == nil)
        #expect(worktreeRoot == nil)
    }

    @Test func extractProjectInfoMalformedJSON() throws {
        let dir = NSTemporaryDirectory() + UUID().uuidString
        try FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(atPath: dir) }

        let jsonl = "not valid json\n"
        let fileName = "session.jsonl"
        try jsonl.write(toFile: "\(dir)/\(fileName)", atomically: true, encoding: .utf8)

        let (path, name, worktreeRoot) = RepoProvider.extractProjectInfoWithWorktree(dirPath: dir, fileName: fileName)
        #expect(path == nil)
        #expect(name == nil)
        #expect(worktreeRoot == nil)
    }

    // MARK: - extractProjectInfoWithWorktree: worktree path detection

    @Test func extractProjectInfoWorktreePath() throws {
        let dir = NSTemporaryDirectory() + UUID().uuidString
        try FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(atPath: dir) }

        let jsonl = #"{"type":"user","cwd":"/Users/dev/my-project/.claude/worktrees/fix-bug","message":{"content":"hello"}}"# + "\n"
        let fileName = "session.jsonl"
        try jsonl.write(toFile: "\(dir)/\(fileName)", atomically: true, encoding: .utf8)

        let (path, name, worktreeRoot) = RepoProvider.extractProjectInfoWithWorktree(dirPath: dir, fileName: fileName)
        #expect(path == "/Users/dev/my-project/.claude/worktrees/fix-bug")
        #expect(name == "dev/my-project")
        #expect(worktreeRoot == "/Users/dev/my-project")
    }

    @Test func extractProjectInfoNonexistentFile() {
        let (path, name, worktreeRoot) = RepoProvider.extractProjectInfoWithWorktree(dirPath: "/nonexistent", fileName: "nope.jsonl")
        #expect(path == nil)
        #expect(name == nil)
        #expect(worktreeRoot == nil)
    }

    @Test func extractProjectInfoSkipsNonUserMessages() throws {
        let dir = NSTemporaryDirectory() + UUID().uuidString
        try FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(atPath: dir) }

        let jsonl = """
        {"type":"assistant","message":{"content":"hi"}}
        {"type":"system","message":{"content":"system"}}
        {"type":"user","cwd":"/Users/dev/org/repo","message":{"content":"hello"}}
        """ + "\n"
        let fileName = "session.jsonl"
        try jsonl.write(toFile: "\(dir)/\(fileName)", atomically: true, encoding: .utf8)

        let (path, name, _) = RepoProvider.extractProjectInfoWithWorktree(dirPath: dir, fileName: fileName)
        #expect(path == "/Users/dev/org/repo")
        #expect(name == "org/repo")
    }

    // MARK: - projectNameFromPath

    @Test func projectNameFromPathTwoComponents() {
        let name = RepoProvider.projectNameFromPath("/Users/dev/my-org/my-project")
        #expect(name == "my-org/my-project")
    }

    @Test func projectNameFromPathSingleComponent() {
        let name = RepoProvider.projectNameFromPath("/project")
        #expect(name == "project")
    }

    @Test func projectNameFromPathDeepPath() {
        let name = RepoProvider.projectNameFromPath("/a/b/c/d/e")
        #expect(name == "d/e")
    }

    @Test func projectNameFromPathEmptyString() {
        let name = RepoProvider.projectNameFromPath("")
        #expect(name == "")
    }

    // MARK: - RepoSession.worktreeLabel

    @Test func repoSessionWithWorktreeLabel() {
        let session = RepoSession(
            sessionId: "wt-session",
            lastModified: Date(),
            gitBranch: "fix-auth",
            worktreeLabel: "fix-auth-worktree",
        )
        #expect(session.worktreeLabel == "fix-auth-worktree")
    }

    // MARK: - RepoInfo computed properties

    @Test func repoInfoSessionAndWorktreeCounts() {
        let sessions = [
            RepoSession(sessionId: "a", lastModified: Date(), gitBranch: "main", worktreeLabel: nil),
            RepoSession(sessionId: "b", lastModified: Date(), gitBranch: "fix", worktreeLabel: "wt1"),
        ]
        let info = RepoInfo(
            projectPath: "/test",
            projectName: "test/repo",
            sessions: sessions,
            worktreePaths: ["wt1", "wt2"],
        )
        #expect(info.sessionCount == 2)
        #expect(info.worktreeCount == 2)
        #expect(info.id == "/test")
    }

    // MARK: - extractGitBranch

    @Test func extractGitBranchValid() throws {
        let tmp = NSTemporaryDirectory() + UUID().uuidString + ".jsonl"
        defer { try? FileManager.default.removeItem(atPath: tmp) }

        let jsonl = #"{"type":"user","gitBranch":"feature/test","message":{"content":"hello"}}"# + "\n"
        try jsonl.write(toFile: tmp, atomically: true, encoding: .utf8)

        let branch = RepoProvider.extractGitBranch(filePath: tmp)
        #expect(branch == "feature/test")
    }

    @Test func extractGitBranchMissing() throws {
        let tmp = NSTemporaryDirectory() + UUID().uuidString + ".jsonl"
        defer { try? FileManager.default.removeItem(atPath: tmp) }

        let jsonl = #"{"type":"user","message":{"content":"hello"}}"# + "\n"
        try jsonl.write(toFile: tmp, atomically: true, encoding: .utf8)

        let branch = RepoProvider.extractGitBranch(filePath: tmp)
        #expect(branch == nil)
    }

    @Test func extractGitBranchEmptyFile() throws {
        let tmp = NSTemporaryDirectory() + UUID().uuidString + ".jsonl"
        defer { try? FileManager.default.removeItem(atPath: tmp) }

        try "".write(toFile: tmp, atomically: true, encoding: .utf8)

        let branch = RepoProvider.extractGitBranch(filePath: tmp)
        #expect(branch == nil)
    }

    @Test func extractGitBranchNonexistent() {
        let branch = RepoProvider.extractGitBranch(filePath: "/nonexistent/file.jsonl")
        #expect(branch == nil)
    }
}
