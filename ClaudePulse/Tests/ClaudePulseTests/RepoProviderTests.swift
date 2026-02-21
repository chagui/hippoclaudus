import Foundation
import Testing

@testable import ClaudePulse

@Suite struct RepoProviderTests {

    // MARK: - RepoSession.timeAgo

    @Test func timeAgoLessThanOneMinute() {
        let session = RepoSession(
            sessionId: "abc",
            lastModified: Date().addingTimeInterval(-30),
            gitBranch: "main"
        )
        #expect(session.timeAgo == "<1m ago")
    }

    @Test func timeAgoMinutes() {
        let session = RepoSession(
            sessionId: "abc",
            lastModified: Date().addingTimeInterval(-300),
            gitBranch: "main"
        )
        #expect(session.timeAgo == "5m ago")
    }

    @Test func timeAgoHours() {
        let session = RepoSession(
            sessionId: "abc",
            lastModified: Date().addingTimeInterval(-7200),
            gitBranch: "main"
        )
        #expect(session.timeAgo == "2h ago")
    }

    @Test func timeAgoDays() {
        let session = RepoSession(
            sessionId: "abc",
            lastModified: Date().addingTimeInterval(-172800),
            gitBranch: "main"
        )
        #expect(session.timeAgo == "2d ago")
    }

    @Test func timeAgoMonths() {
        let session = RepoSession(
            sessionId: "abc",
            lastModified: Date().addingTimeInterval(-86400 * 45),
            gitBranch: "main"
        )
        #expect(session.timeAgo == "1mo ago")
    }

    // MARK: - RepoSession.shortId

    @Test func shortId() {
        let session = RepoSession(
            sessionId: "abcdefghijklmnop",
            lastModified: Date(),
            gitBranch: "main"
        )
        #expect(session.shortId == "abcdefgh")
    }

    @Test func shortIdShortSession() {
        let session = RepoSession(
            sessionId: "abc",
            lastModified: Date(),
            gitBranch: "main"
        )
        #expect(session.shortId == "abc")
    }

    // MARK: - extractProjectInfo

    @Test func extractProjectInfoValid() {
        let dir = NSTemporaryDirectory() + UUID().uuidString
        try! FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(atPath: dir) }

        let jsonl = #"{"type":"user","cwd":"/Users/dev/my-org/my-project","message":{"content":"hello"}}"# + "\n"
        let fileName = "session.jsonl"
        try! jsonl.write(toFile: "\(dir)/\(fileName)", atomically: true, encoding: .utf8)

        let (path, name) = RepoProvider.extractProjectInfo(dirPath: dir, fileName: fileName)
        #expect(path == "/Users/dev/my-org/my-project")
        #expect(name == "my-org/my-project")
    }

    @Test func extractProjectInfoMissingCwd() {
        let dir = NSTemporaryDirectory() + UUID().uuidString
        try! FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(atPath: dir) }

        let jsonl = #"{"type":"user","message":{"content":"hello"}}"# + "\n"
        let fileName = "session.jsonl"
        try! jsonl.write(toFile: "\(dir)/\(fileName)", atomically: true, encoding: .utf8)

        let (path, name) = RepoProvider.extractProjectInfo(dirPath: dir, fileName: fileName)
        #expect(path == nil)
        #expect(name == nil)
    }

    @Test func extractProjectInfoMalformedJSON() {
        let dir = NSTemporaryDirectory() + UUID().uuidString
        try! FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(atPath: dir) }

        let jsonl = "not valid json\n"
        let fileName = "session.jsonl"
        try! jsonl.write(toFile: "\(dir)/\(fileName)", atomically: true, encoding: .utf8)

        let (path, name) = RepoProvider.extractProjectInfo(dirPath: dir, fileName: fileName)
        #expect(path == nil)
        #expect(name == nil)
    }

    // MARK: - extractGitBranch

    @Test func extractGitBranchValid() {
        let tmp = NSTemporaryDirectory() + UUID().uuidString + ".jsonl"
        defer { try? FileManager.default.removeItem(atPath: tmp) }

        let jsonl = #"{"type":"user","gitBranch":"feature/test","message":{"content":"hello"}}"# + "\n"
        try! jsonl.write(toFile: tmp, atomically: true, encoding: .utf8)

        let branch = RepoProvider.extractGitBranch(filePath: tmp)
        #expect(branch == "feature/test")
    }

    @Test func extractGitBranchMissing() {
        let tmp = NSTemporaryDirectory() + UUID().uuidString + ".jsonl"
        defer { try? FileManager.default.removeItem(atPath: tmp) }

        let jsonl = #"{"type":"user","message":{"content":"hello"}}"# + "\n"
        try! jsonl.write(toFile: tmp, atomically: true, encoding: .utf8)

        let branch = RepoProvider.extractGitBranch(filePath: tmp)
        #expect(branch == nil)
    }

    @Test func extractGitBranchEmptyFile() {
        let tmp = NSTemporaryDirectory() + UUID().uuidString + ".jsonl"
        defer { try? FileManager.default.removeItem(atPath: tmp) }

        try! "".write(toFile: tmp, atomically: true, encoding: .utf8)

        let branch = RepoProvider.extractGitBranch(filePath: tmp)
        #expect(branch == nil)
    }

    @Test func extractGitBranchNonexistent() {
        let branch = RepoProvider.extractGitBranch(filePath: "/nonexistent/file.jsonl")
        #expect(branch == nil)
    }
}
