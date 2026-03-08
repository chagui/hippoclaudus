import Foundation
import Testing

@testable import Hippo

@Suite struct AdversarialTests {

    // MARK: - Helpers

    private func writeTempFile(_ data: Data, ext: String = "md") -> String {
        let tmp = NSTemporaryDirectory() + UUID().uuidString + "." + ext
        FileManager.default.createFile(atPath: tmp, contents: data)
        return tmp
    }

    private func writeTempFile(_ content: String, ext: String = "md") -> String {
        writeTempFile(content.data(using: .utf8)!, ext: ext)
    }

    // MARK: - Frontmatter parser: adversarial inputs

    @Test func frontmatterRandomBytes() {
        for _ in 0..<20 {
            let size = Int.random(in: 0...2048)
            var bytes = [UInt8](repeating: 0, count: size)
            for i in 0..<size { bytes[i] = UInt8.random(in: 0...255) }
            let path = writeTempFile(Data(bytes))
            defer { try? FileManager.default.removeItem(atPath: path) }

            // Should not crash
            let _ = SearchProvider.extractFrontmatter(filePath: path)
            let _ = VaultTagProvider.extractFrontmatter(filePath: path)
        }
    }

    @Test func frontmatterDeeplyNested() {
        var content = "---\ntitle: Deep\ntags:\n"
        for i in 0..<100 {
            content += "  - tag\(i)\n"
        }
        content += "---\n"
        let path = writeTempFile(content)
        defer { try? FileManager.default.removeItem(atPath: path) }

        let (title, tags) = SearchProvider.extractFrontmatter(filePath: path)
        #expect(title == "Deep")
        #expect(tags.count > 0)
    }

    @Test func frontmatterHugeLines() {
        let hugeLine = String(repeating: "x", count: 100_000)
        let content = "---\ntitle: \(hugeLine)\ntags:\n  - tag1\n---\n"
        let path = writeTempFile(content)
        defer { try? FileManager.default.removeItem(atPath: path) }

        // Should not crash
        let (title, _) = SearchProvider.extractFrontmatter(filePath: path)
        let _ = title
    }

    @Test func frontmatterBinaryData() {
        var data = Data([0xFF, 0xFE, 0x00, 0x01])
        data.append("---\ntitle: test\n---\n".data(using: .utf8)!)
        let path = writeTempFile(data)
        defer { try? FileManager.default.removeItem(atPath: path) }

        // Should not crash
        let (_, _) = SearchProvider.extractFrontmatter(filePath: path)
    }

    // MARK: - JSONL parsers: adversarial inputs

    @Test func extractProjectInfoMalformedJSONL() {
        let dir = NSTemporaryDirectory() + UUID().uuidString
        try! FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(atPath: dir) }

        let malformedInputs = [
            "",
            "not json at all",
            "{{{",
            #"{"type":"user"}"#,
            String(repeating: "{", count: 10000),
            #"{"type":"user","cwd":null}"#,
            #"{"type":"user","cwd":12345}"#,
        ]

        for (i, input) in malformedInputs.enumerated() {
            let fileName = "test\(i).jsonl"
            try! input.write(toFile: "\(dir)/\(fileName)", atomically: true, encoding: .utf8)
            // Should not crash
            let (_, _, _) = RepoProvider.extractProjectInfoWithWorktree(dirPath: dir, fileName: fileName)
        }
    }

    @Test func extractGitBranchMalformedJSONL() {
        let malformedInputs = [
            "",
            "not json",
            "{{{",
            #"{"type":"user"}"#,
            #"{"type":"user","gitBranch":null}"#,
            #"{"type":"user","gitBranch":42}"#,
        ]

        for input in malformedInputs {
            let tmp = NSTemporaryDirectory() + UUID().uuidString + ".jsonl"
            try! input.write(toFile: tmp, atomically: true, encoding: .utf8)
            defer { try? FileManager.default.removeItem(atPath: tmp) }

            // Should not crash
            let _ = RepoProvider.extractGitBranch(filePath: tmp)
        }
    }

    // MARK: - Worktree path edge cases

    @Test func extractProjectInfoWorktreeEdgeCases() {
        let dir = NSTemporaryDirectory() + UUID().uuidString
        try! FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(atPath: dir) }

        let edgeCases = [
            // Worktree marker at various positions
            #"{"type":"user","cwd":"/.claude/worktrees/x","message":{"content":"hi"}}"#,
            // Double worktree marker
            #"{"type":"user","cwd":"/a/.claude/worktrees/b/.claude/worktrees/c","message":{"content":"hi"}}"#,
            // Worktree marker with empty name (trailing slash)
            #"{"type":"user","cwd":"/project/.claude/worktrees/","message":{"content":"hi"}}"#,
            // Very long worktree name
            #"{"type":"user","cwd":"/project/.claude/worktrees/"# + String(repeating: "a", count: 1000) + #"","message":{"content":"hi"}}"#,
            // Unicode in path
            #"{"type":"user","cwd":"/Users/日本語/.claude/worktrees/修正","message":{"content":"hi"}}"#,
        ]

        for (i, input) in edgeCases.enumerated() {
            let fileName = "wt-edge\(i).jsonl"
            try! (input + "\n").write(toFile: "\(dir)/\(fileName)", atomically: true, encoding: .utf8)
            // Should not crash
            let (_, _, _) = RepoProvider.extractProjectInfoWithWorktree(dirPath: dir, fileName: fileName)
        }
    }

    @Test func projectNameFromPathAdversarial() {
        let inputs = [
            "",
            "/",
            "//",
            String(repeating: "/a", count: 1000),
            "/\u{0000}",
            "no-slashes-at-all",
        ]
        for input in inputs {
            // Should not crash
            let _ = RepoProvider.projectNameFromPath(input)
        }
    }

    // MARK: - ActiveSessionResponse computed properties: edge cases

    @Test func timeSplitAndToolSummaryWithExtremeValues() throws {
        let json: [String: Any] = [
            "session_id": "extreme-optional", "project_name": "test",
            "project_cwd": "/tmp", "git_branch": "main",
            "started_at": "2025-01-01T00:00:00Z", "exchange_count": 0,
            "model": "sonnet", "total_input_tokens": 0,
            "total_output_tokens": 0, "total_cache_read_tokens": 0,
            "total_cache_creation_tokens": 0, "avg_turn_duration_ms": 0,
            "turn_count": 0, "state": "active",
            "agent_time_pct": 100.0, "user_time_pct": 0.0,
            "write_count": 0, "edit_count": 0, "bash_count": 0,
            "files_touched_count": 0,
        ]
        let data = try JSONSerialization.data(withJSONObject: json)
        let session = try JSONDecoder().decode(ActiveSessionResponse.self, from: data)

        #expect(session.timeSplit == "Agent 100% / User 0%")
        #expect(session.toolSummary == "0W 0E 0B")
        #expect(session.filesTouchedCount == 0)
    }

    // MARK: - ActiveSessionResponse: extreme values

    @Test func extremeTokenValues() throws {
        let json: [String: Any] = [
            "session_id": "extreme",
            "project_name": "test",
            "project_cwd": "/tmp",
            "git_branch": "main",
            "started_at": "2025-01-01T00:00:00Z",
            "exchange_count": 0,
            "model": "claude-sonnet-4-6",
            "total_input_tokens": Int.max / 2,
            "total_output_tokens": Int.max / 2,
            "total_cache_read_tokens": 0,
            "total_cache_creation_tokens": 0,
            "avg_turn_duration_ms": Int.max,
            "turn_count": Int.max,
            "state": "active",
        ]

        let data = try JSONSerialization.data(withJSONObject: json)
        let session = try JSONDecoder().decode(ActiveSessionResponse.self, from: data)

        // Should not crash
        let _ = session.totalTokensFormatted
        let _ = session.estimatedCost
        let _ = session.avgTurnDuration
        let _ = session.modelLabel
    }

    @Test func emptyModelString() throws {
        let json: [String: Any] = [
            "session_id": "empty-model",
            "project_name": "test",
            "project_cwd": "/tmp",
            "git_branch": "main",
            "started_at": "2025-01-01T00:00:00Z",
            "exchange_count": 0,
            "model": "",
            "total_input_tokens": 1000,
            "total_output_tokens": 500,
            "total_cache_read_tokens": 0,
            "total_cache_creation_tokens": 0,
            "avg_turn_duration_ms": 0,
            "turn_count": 0,
            "state": "active",
        ]

        let data = try JSONSerialization.data(withJSONObject: json)
        let session = try JSONDecoder().decode(ActiveSessionResponse.self, from: data)

        #expect(session.modelLabel == "")
        let _ = session.estimatedCost
    }

    // MARK: - Malformed ISO8601 dates

    @Test func malformedDates() throws {
        let badDates = [
            "not-a-date",
            "",
            "2025",
            "2025-13-45T99:99:99Z",
            "\u{1F389}",
            String(repeating: "2025-01-01T00:00:00Z", count: 1000),
        ]

        for badDate in badDates {
            let json: [String: Any] = [
                "session_id": "bad-date",
                "project_name": "test",
                "project_cwd": "/tmp",
                "git_branch": "main",
                "started_at": badDate,
                "exchange_count": 0,
                "model": "sonnet",
                "total_input_tokens": 0,
                "total_output_tokens": 0,
                "total_cache_read_tokens": 0,
                "total_cache_creation_tokens": 0,
                "avg_turn_duration_ms": 0,
                "turn_count": 0,
                "state": "active",
            ]

            let data = try JSONSerialization.data(withJSONObject: json)
            let session = try JSONDecoder().decode(ActiveSessionResponse.self, from: data)

            // Should not crash
            let _ = session.duration
        }
    }
}
