import Foundation
import Testing

@testable import ClaudePulse

@Suite struct StatusResponseDecodingTests {

    @Test func decodeValidStatusJSON() throws {
        let json = """
        {
            "last_sync_ago": "5m ago",
            "sessions_unprocessed": 3,
            "sessions_processed": 10,
            "vault_notes": 42,
            "active_session_count": 1,
            "active_sessions": [
                {
                    "session_id": "abc-123",
                    "project_name": "test/project",
                    "project_cwd": "/home/user/test/project",
                    "git_branch": "main",
                    "started_at": "2025-01-01T00:00:00Z",
                    "exchange_count": 5,
                    "model": "claude-sonnet-4-6",
                    "total_input_tokens": 50000,
                    "total_output_tokens": 20000,
                    "total_cache_read_tokens": 10000,
                    "total_cache_creation_tokens": 5000,
                    "avg_turn_duration_ms": 15000,
                    "turn_count": 5,
                    "state": "active"
                }
            ]
        }
        """

        let data = json.data(using: .utf8)!
        let status = try JSONDecoder().decode(StatusResponse.self, from: data)

        #expect(status.lastSyncAgo == "5m ago")
        #expect(status.unprocessed == 3)
        #expect(status.processed == 10)
        #expect(status.vaultNotes == 42)
        #expect(status.activeSessionCount == 1)
        #expect(status.activeSessions?.count == 1)

        let session = status.activeSessions!.first!
        #expect(session.sessionId == "abc-123")
        #expect(session.model == "claude-sonnet-4-6")
        #expect(session.totalInputTokens == 50000)
    }

    @Test func decodeWithMissingOptionalFields() throws {
        let json = "{}"
        let data = json.data(using: .utf8)!
        let status = try JSONDecoder().decode(StatusResponse.self, from: data)

        #expect(status.lastSyncAgo == nil)
        #expect(status.unprocessed == nil)
        #expect(status.processed == nil)
        #expect(status.vaultNotes == nil)
        #expect(status.activeSessionCount == nil)
        #expect(status.activeSessions == nil)
    }

    @Test func decodeWithEmptyActiveSessions() throws {
        let json = """
        {
            "last_sync_ago": "1h ago",
            "sessions_processed": 5,
            "active_sessions": []
        }
        """

        let data = json.data(using: .utf8)!
        let status = try JSONDecoder().decode(StatusResponse.self, from: data)

        #expect(status.lastSyncAgo == "1h ago")
        #expect(status.activeSessions?.count == 0)
    }

    @Test func activeSessionResponseDecoding() throws {
        let json = """
        {
            "session_id": "test-id",
            "project_name": "org/repo",
            "project_cwd": "/path/to/repo",
            "git_branch": "feature",
            "started_at": "2025-06-15T10:30:00Z",
            "exchange_count": 10,
            "model": "claude-opus-4-6",
            "total_input_tokens": 200000,
            "total_output_tokens": 100000,
            "total_cache_read_tokens": 50000,
            "total_cache_creation_tokens": 25000,
            "avg_turn_duration_ms": 30000,
            "turn_count": 8,
            "state": "waiting"
        }
        """

        let data = json.data(using: .utf8)!
        let session = try JSONDecoder().decode(ActiveSessionResponse.self, from: data)

        #expect(session.id == "test-id")
        #expect(session.projectName == "org/repo")
        #expect(session.isWaiting == true)
        #expect(session.modelLabel == "Opus")
    }
}
