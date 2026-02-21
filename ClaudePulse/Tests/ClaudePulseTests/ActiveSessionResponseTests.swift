import Foundation
import Testing

@testable import ClaudePulse

@Suite struct ActiveSessionResponseTests {

    // MARK: - Factory helper

    private func makeSession(
        model: String = "claude-sonnet-4-6",
        totalInputTokens: Int = 10000,
        totalOutputTokens: Int = 5000,
        totalCacheReadTokens: Int = 0,
        totalCacheCreationTokens: Int = 0,
        avgTurnDurationMs: Int = 15000,
        turnCount: Int = 5,
        startedAt: String = "2025-01-01T00:00:00Z",
        exchangeCount: Int = 3,
        state: String = "active"
    ) -> ActiveSessionResponse {
        let json: [String: Any] = [
            "session_id": "test-session-123",
            "project_name": "test/project",
            "project_cwd": "/home/user/test/project",
            "git_branch": "main",
            "started_at": startedAt,
            "exchange_count": exchangeCount,
            "model": model,
            "total_input_tokens": totalInputTokens,
            "total_output_tokens": totalOutputTokens,
            "total_cache_read_tokens": totalCacheReadTokens,
            "total_cache_creation_tokens": totalCacheCreationTokens,
            "avg_turn_duration_ms": avgTurnDurationMs,
            "turn_count": turnCount,
            "state": state,
        ]
        let data = try! JSONSerialization.data(withJSONObject: json)
        return try! JSONDecoder().decode(ActiveSessionResponse.self, from: data)
    }

    // MARK: - modelLabel tests

    @Test func modelLabelOpus() {
        let s = makeSession(model: "claude-opus-4-6")
        #expect(s.modelLabel == "Opus")
    }

    @Test func modelLabelSonnet() {
        let s = makeSession(model: "claude-sonnet-4-6")
        #expect(s.modelLabel == "Sonnet")
    }

    @Test func modelLabelHaiku() {
        let s = makeSession(model: "claude-haiku-4-5-20251001")
        #expect(s.modelLabel == "Haiku")
    }

    @Test func modelLabelUnknown() {
        let s = makeSession(model: "some-future-model")
        #expect(s.modelLabel == "some-future-model")
    }

    // MARK: - totalTokensFormatted tests

    @Test func totalTokensPlain() {
        let s = makeSession(totalInputTokens: 500, totalOutputTokens: 300)
        #expect(s.totalTokensFormatted == "800")
    }

    @Test func totalTokensK() {
        let s = makeSession(totalInputTokens: 5000, totalOutputTokens: 5000)
        #expect(s.totalTokensFormatted == "10K")
    }

    @Test func totalTokensM() {
        let s = makeSession(totalInputTokens: 800_000, totalOutputTokens: 400_000)
        #expect(s.totalTokensFormatted == "1.2M")
    }

    // MARK: - estimatedCost tests

    @Test func estimatedCostZeroTokens() {
        let s = makeSession(totalInputTokens: 0, totalOutputTokens: 0)
        #expect(s.estimatedCost == "<$0.01")
    }

    @Test func estimatedCostOpusMoreExpensive() {
        let opus = makeSession(model: "claude-opus-4-6", totalInputTokens: 100_000, totalOutputTokens: 50_000)
        let sonnet = makeSession(model: "claude-sonnet-4-6", totalInputTokens: 100_000, totalOutputTokens: 50_000)
        let haiku = makeSession(model: "claude-haiku-4-5", totalInputTokens: 100_000, totalOutputTokens: 50_000)

        let opusCost = parseCost(opus.estimatedCost)
        let sonnetCost = parseCost(sonnet.estimatedCost)
        let haikuCost = parseCost(haiku.estimatedCost)

        #expect(opusCost > sonnetCost, "Opus should cost more than Sonnet")
        #expect(sonnetCost > haikuCost, "Sonnet should cost more than Haiku")
    }

    @Test func estimatedCostMonotonicallyIncreasesWithTokens() {
        let low = makeSession(totalInputTokens: 10_000, totalOutputTokens: 5_000)
        let high = makeSession(totalInputTokens: 100_000, totalOutputTokens: 50_000)

        let lowCost = parseCost(low.estimatedCost)
        let highCost = parseCost(high.estimatedCost)

        #expect(highCost >= lowCost)
    }

    // MARK: - avgTurnDuration tests

    @Test func avgTurnDurationZeroTurns() {
        let s = makeSession(avgTurnDurationMs: 0, turnCount: 0)
        #expect(s.avgTurnDuration == "-")
    }

    @Test func avgTurnDurationSeconds() {
        let s = makeSession(avgTurnDurationMs: 15000, turnCount: 1)
        #expect(s.avgTurnDuration == "15s")
    }

    @Test func avgTurnDurationMinutesAndSeconds() {
        let s = makeSession(avgTurnDurationMs: 90_000, turnCount: 1)
        #expect(s.avgTurnDuration == "1m 30s")
    }

    // MARK: - duration tests

    @Test func durationWithValidISO8601() {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime]
        let twoHoursAgo = Date().addingTimeInterval(-7200)
        let ts = formatter.string(from: twoHoursAgo)

        let s = makeSession(startedAt: ts)
        #expect(s.duration.contains("h"), "Duration should contain hours: \(s.duration)")
    }

    @Test func durationWithInvalidTimestamp() {
        let s = makeSession(startedAt: "not-a-date")
        #expect(s.duration == "?")
    }

    @Test func durationRecentShowsLessThanOneMinute() {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime]
        let tenSecondsAgo = Date().addingTimeInterval(-10)
        let ts = formatter.string(from: tenSecondsAgo)

        let s = makeSession(startedAt: ts)
        #expect(s.duration == "<1m")
    }

    // MARK: - isWaiting tests

    @Test func isWaitingTrue() {
        let s = makeSession(state: "waiting")
        #expect(s.isWaiting)
    }

    @Test func isWaitingFalse() {
        let s = makeSession(state: "active")
        #expect(!s.isWaiting)
    }

    // MARK: - formatDuration tests

    @Test func formatDurationLessThanMinute() {
        let start = Date().addingTimeInterval(-30)
        let result = ActiveSessionResponse.formatDuration(since: start)
        #expect(result == "<1m")
    }

    @Test func formatDurationMinutes() {
        let start = Date().addingTimeInterval(-300)
        let result = ActiveSessionResponse.formatDuration(since: start)
        #expect(result == "5m")
    }

    @Test func formatDurationHoursAndMinutes() {
        let start = Date().addingTimeInterval(-3900)
        let result = ActiveSessionResponse.formatDuration(since: start)
        #expect(result == "1h 5m")
    }

    // MARK: - Helpers

    private func parseCost(_ cost: String) -> Double {
        if cost == "<$0.01" { return 0.005 }
        return Double(cost.replacingOccurrences(of: "$", with: "")) ?? 0
    }
}
