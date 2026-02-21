@preconcurrency import Foundation
import SwiftUI

@MainActor
final class StatusProvider: ObservableObject {
    @Published var lastSyncAgo: String = "Unknown"
    @Published var unprocessed: Int = 0
    @Published var processed: Int = 0
    @Published var vaultNotes: Int = 0
    @Published var isSyncing: Bool = false
    @Published var errorMessage: String?
    @Published var activeSessionCount: Int = 0
    @Published var activeSessions: [ActiveSessionResponse] = []

    /// True if any active session is waiting for user input.
    var anySessionWaiting: Bool {
        activeSessions.contains { $0.isWaiting }
    }

    let enrichmentProvider: GitEnrichmentProvider
    let repoProvider: RepoProvider
    let vaultTagProvider: VaultTagProvider
    private var refreshTimer: Timer?

    init(
        enrichmentProvider: GitEnrichmentProvider,
        repoProvider: RepoProvider,
        vaultTagProvider: VaultTagProvider
    ) {
        self.enrichmentProvider = enrichmentProvider
        self.repoProvider = repoProvider
        self.vaultTagProvider = vaultTagProvider
        startTimer()
        Task {
            await refresh()
        }
    }

    // Timer is invalidated when the object is deallocated naturally

    func startTimer() {
        refreshTimer?.invalidate()
        refreshTimer = Timer.scheduledTimer(withTimeInterval: 60, repeats: true) { [weak self] _ in
            guard let self else { return }
            Task { @MainActor in
                await self.refresh()
            }
        }
    }

    func refresh() async {
        do {
            let data = try await CLIRunner.run(arguments: ["status"])
            let parsed = try JSONDecoder().decode(StatusResponse.self, from: data)
            self.lastSyncAgo = parsed.lastSyncAgo ?? "Never"
            self.unprocessed = parsed.unprocessed ?? 0
            self.processed = parsed.processed ?? 0
            self.vaultNotes = parsed.vaultNotes ?? 0
            self.activeSessionCount = parsed.activeSessionCount ?? 0
            self.activeSessions = parsed.activeSessions ?? []
            self.errorMessage = nil

            let sessions = self.activeSessions
            Task {
                await self.enrichmentProvider.enrich(sessions: sessions)
                await self.repoProvider.refresh()
                await self.vaultTagProvider.refresh()
            }
        } catch {
            self.errorMessage = error.localizedDescription
        }
    }

    func syncNow() async {
        guard !isSyncing else { return }
        isSyncing = true
        errorMessage = nil
        do {
            _ = try await CLIRunner.run(arguments: ["sync", "--days", "7"])
            await refresh()
        } catch {
            errorMessage = error.localizedDescription
        }
        isSyncing = false
    }
}

struct StatusResponse: Decodable {
    let lastSyncAgo: String?
    let unprocessed: Int?
    let processed: Int?
    let vaultNotes: Int?
    let activeSessionCount: Int?
    let activeSessions: [ActiveSessionResponse]?
    let aggregateStats: AggregateStatsResponse?

    enum CodingKeys: String, CodingKey {
        case lastSyncAgo = "last_sync_ago"
        case unprocessed = "sessions_unprocessed"
        case processed = "sessions_processed"
        case vaultNotes = "vault_notes"
        case activeSessionCount = "active_session_count"
        case activeSessions = "active_sessions"
        case aggregateStats = "aggregate_stats"
    }
}

struct AggregateStatsResponse: Decodable, Sendable {
    let sessions: Int?
    let agentTimeMs: Int?
    let userTimeMs: Int?
    let agentTimePct: Double?
    let userTimePct: Double?
    let writeCount: Int?
    let editCount: Int?
    let bashCount: Int?
    let filesTouchedCount: Int?

    enum CodingKeys: String, CodingKey {
        case sessions
        case agentTimeMs = "agent_time_ms"
        case userTimeMs = "user_time_ms"
        case agentTimePct = "agent_time_pct"
        case userTimePct = "user_time_pct"
        case writeCount = "write_count"
        case editCount = "edit_count"
        case bashCount = "bash_count"
        case filesTouchedCount = "files_touched_count"
    }
}

struct ActiveSessionResponse: Decodable, Identifiable, Sendable {
    let sessionId: String
    let projectName: String
    let projectCwd: String
    let gitBranch: String
    let startedAt: String
    let exchangeCount: Int
    let model: String
    let totalInputTokens: Int
    let totalOutputTokens: Int
    let totalCacheReadTokens: Int
    let totalCacheCreationTokens: Int
    let avgTurnDurationMs: Int
    let turnCount: Int
    let state: String
    // New timing/tool fields (optional for backward compatibility)
    let agentTimeMs: Int?
    let userTimeMs: Int?
    let agentTimePct: Double?
    let userTimePct: Double?
    let writeCount: Int?
    let editCount: Int?
    let bashCount: Int?
    let filesTouchedCount: Int?

    var id: String { sessionId }

    /// Human-friendly model label (e.g. "Opus 4.6", "Sonnet 4.6").
    var modelLabel: String {
        if model.contains("opus") { return "Opus" }
        if model.contains("sonnet") { return "Sonnet" }
        if model.contains("haiku") { return "Haiku" }
        return model
    }

    /// Total tokens (input + output) formatted as "123K" or "1.2M".
    var totalTokensFormatted: String {
        let total = totalInputTokens + totalOutputTokens
        if total >= 1_000_000 {
            return String(format: "%.1fM", Double(total) / 1_000_000)
        } else if total >= 1_000 {
            return String(format: "%.0fK", Double(total) / 1_000)
        }
        return "\(total)"
    }

    /// Estimated cost in USD based on model and token counts.
    var estimatedCost: String {
        // Pricing per million tokens (as of early 2026)
        let (inputPer1M, outputPer1M, cacheReadPer1M, cacheCreatePer1M): (Double, Double, Double, Double) = {
            if model.contains("opus") {
                return (15.0, 75.0, 1.5, 18.75)
            } else if model.contains("sonnet") {
                return (3.0, 15.0, 0.3, 3.75)
            } else if model.contains("haiku") {
                return (0.80, 4.0, 0.08, 1.0)
            }
            return (3.0, 15.0, 0.3, 3.75) // default to Sonnet pricing
        }()

        // Non-cached input = total input - cache_read - cache_creation
        let nonCachedInput = max(0, totalInputTokens - totalCacheReadTokens - totalCacheCreationTokens)
        let cost = (Double(nonCachedInput) * inputPer1M
            + Double(totalOutputTokens) * outputPer1M
            + Double(totalCacheReadTokens) * cacheReadPer1M
            + Double(totalCacheCreationTokens) * cacheCreatePer1M) / 1_000_000

        if cost < 0.01 {
            return "<$0.01"
        }
        return String(format: "$%.2f", cost)
    }

    /// Average turn duration formatted as "Xs" or "Xm Ys".
    var avgTurnDuration: String {
        guard turnCount > 0 else { return "-" }
        let seconds = avgTurnDurationMs / 1000
        if seconds >= 60 {
            return "\(seconds / 60)m \(seconds % 60)s"
        }
        return "\(seconds)s"
    }

    /// Formats elapsed time since session started as "Xh Ym" / "Xm" / "<1m".
    var duration: String {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        guard let start = formatter.date(from: startedAt) else {
            formatter.formatOptions = [.withInternetDateTime]
            guard let start = formatter.date(from: startedAt) else { return "?" }
            return Self.formatDuration(since: start)
        }
        return Self.formatDuration(since: start)
    }

    var isWaiting: Bool { state == "waiting" }

    static func formatDuration(since start: Date) -> String {
        let elapsed = Int(Date().timeIntervalSince(start))
        if elapsed < 60 {
            return "<1m"
        }
        let hours = elapsed / 3600
        let minutes = (elapsed % 3600) / 60
        if hours > 0 {
            return "\(hours)h \(minutes)m"
        }
        return "\(minutes)m"
    }

    /// Agent/user time split formatted for display.
    var timeSplit: String? {
        guard let agentPct = agentTimePct, let userPct = userTimePct else { return nil }
        return String(format: "Agent %.0f%% / User %.0f%%", agentPct, userPct)
    }

    /// Tool usage summary.
    var toolSummary: String? {
        guard let w = writeCount, let e = editCount, let b = bashCount else { return nil }
        return "\(w)W \(e)E \(b)B"
    }

    enum CodingKeys: String, CodingKey {
        case sessionId = "session_id"
        case projectName = "project_name"
        case projectCwd = "project_cwd"
        case gitBranch = "git_branch"
        case startedAt = "started_at"
        case exchangeCount = "exchange_count"
        case model
        case totalInputTokens = "total_input_tokens"
        case totalOutputTokens = "total_output_tokens"
        case totalCacheReadTokens = "total_cache_read_tokens"
        case totalCacheCreationTokens = "total_cache_creation_tokens"
        case avgTurnDurationMs = "avg_turn_duration_ms"
        case turnCount = "turn_count"
        case state
        case agentTimeMs = "agent_time_ms"
        case userTimeMs = "user_time_ms"
        case agentTimePct = "agent_time_pct"
        case userTimePct = "user_time_pct"
        case writeCount = "write_count"
        case editCount = "edit_count"
        case bashCount = "bash_count"
        case filesTouchedCount = "files_touched_count"
    }
}
