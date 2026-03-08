import Foundation
import Testing

@testable import Hippo

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

    // MARK: - AggregateStats decoding

    @Test func decodeWithAggregateStats() throws {
        let json = """
        {
            "last_sync_ago": "2h ago",
            "sessions_processed": 15,
            "aggregate_stats": {
                "sessions": 15,
                "agent_time_ms": 300000,
                "user_time_ms": 120000,
                "agent_time_pct": 71.4,
                "user_time_pct": 28.6,
                "write_count": 50,
                "edit_count": 120,
                "bash_count": 80,
                "files_touched_count": 95
            }
        }
        """
        let data = json.data(using: .utf8)!
        let status = try JSONDecoder().decode(StatusResponse.self, from: data)

        let stats = try #require(status.aggregateStats)
        #expect(stats.sessions == 15)
        #expect(stats.agentTimeMs == 300000)
        #expect(stats.userTimeMs == 120000)
        #expect(stats.agentTimePct == 71.4)
        #expect(stats.userTimePct == 28.6)
        #expect(stats.writeCount == 50)
        #expect(stats.editCount == 120)
        #expect(stats.bashCount == 80)
        #expect(stats.filesTouchedCount == 95)
    }

    @Test func decodeAggregateStatsWithMissingFields() throws {
        let json = """
        {
            "aggregate_stats": {
                "sessions": 5
            }
        }
        """
        let data = json.data(using: .utf8)!
        let status = try JSONDecoder().decode(StatusResponse.self, from: data)

        let stats = try #require(status.aggregateStats)
        #expect(stats.sessions == 5)
        #expect(stats.agentTimeMs == nil)
        #expect(stats.writeCount == nil)
        #expect(stats.filesTouchedCount == nil)
    }

    @Test func decodeAggregateStatsNilWhenMissing() throws {
        let json = """
        {
            "last_sync_ago": "1h ago"
        }
        """
        let data = json.data(using: .utf8)!
        let status = try JSONDecoder().decode(StatusResponse.self, from: data)
        #expect(status.aggregateStats == nil)
    }

    // MARK: - ActiveSessionResponse with optional fields

    @Test func activeSessionWithAllOptionalFields() throws {
        let json = """
        {
            "session_id": "full-session",
            "project_name": "test",
            "project_cwd": "/tmp",
            "git_branch": "main",
            "started_at": "2025-01-01T00:00:00Z",
            "exchange_count": 10,
            "model": "claude-opus-4-6",
            "total_input_tokens": 100000,
            "total_output_tokens": 50000,
            "total_cache_read_tokens": 20000,
            "total_cache_creation_tokens": 10000,
            "avg_turn_duration_ms": 25000,
            "turn_count": 8,
            "state": "active",
            "agent_time_ms": 180000,
            "user_time_ms": 60000,
            "agent_time_pct": 75.0,
            "user_time_pct": 25.0,
            "write_count": 5,
            "edit_count": 15,
            "bash_count": 8,
            "files_touched_count": 12,
            "worktree_root": "/Users/dev/project"
        }
        """
        let data = json.data(using: .utf8)!
        let s = try JSONDecoder().decode(ActiveSessionResponse.self, from: data)

        #expect(s.agentTimeMs == 180000)
        #expect(s.userTimeMs == 60000)
        #expect(s.agentTimePct == 75.0)
        #expect(s.userTimePct == 25.0)
        #expect(s.writeCount == 5)
        #expect(s.editCount == 15)
        #expect(s.bashCount == 8)
        #expect(s.filesTouchedCount == 12)
        #expect(s.worktreeRoot == "/Users/dev/project")
        #expect(s.timeSplit == "Agent 75% / User 25%")
        #expect(s.toolSummary == "5W 15E 8B")
    }
}
