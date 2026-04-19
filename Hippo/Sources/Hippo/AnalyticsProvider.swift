import Foundation

// MARK: - Wire types (mirror `hpc analytics --json`)

struct AnalyticsDistribution: Decodable {
    let count: Int
    let min: Double
    let max: Double
    let mean: Double
    let p50: Double
    let p90: Double
    let p95: Double
    let p99: Double
}

struct AnalyticsTotals: Decodable {
    let costUsd: Double
    let totalTokens: Int
    let totalDurationMs: Int

    enum CodingKeys: String, CodingKey {
        case costUsd = "cost_usd"
        case totalTokens = "total_tokens"
        case totalDurationMs = "total_duration_ms"
    }
}

struct AnalyticsModelBucket: Decodable, Identifiable {
    let model: String
    let sessionCount: Int
    let costUsd: Double
    let totalTokens: Int
    let totalDurationMs: Int

    var id: String {
        model
    }

    enum CodingKeys: String, CodingKey {
        case model
        case sessionCount = "session_count"
        case costUsd = "cost_usd"
        case totalTokens = "total_tokens"
        case totalDurationMs = "total_duration_ms"
    }
}

struct AnalyticsDailyBucket: Decodable, Identifiable {
    let date: String
    let model: String
    let sessionCount: Int
    let costUsd: Double
    let totalTokens: Int
    let agentTimeMs: Int
    let userTimeMs: Int

    var id: String {
        "\(date)-\(model)"
    }

    enum CodingKeys: String, CodingKey {
        case date
        case model
        case sessionCount = "session_count"
        case costUsd = "cost_usd"
        case totalTokens = "total_tokens"
        case agentTimeMs = "agent_time_ms"
        case userTimeMs = "user_time_ms"
    }
}

struct AnalyticsWindow: Decodable {
    let start: String
    let end: String
}

struct AnalyticsResponse: Decodable {
    let days: Int
    let sessionCount: Int
    let window: AnalyticsWindow
    let totals: AnalyticsTotals
    let distributions: [String: AnalyticsDistribution]
    let byModel: [AnalyticsModelBucket]
    let daily: [AnalyticsDailyBucket]

    enum CodingKeys: String, CodingKey {
        case days
        case sessionCount = "session_count"
        case window
        case totals
        case distributions
        case byModel = "by_model"
        case daily
    }
}

// MARK: - Provider

@MainActor
final class AnalyticsProvider: ObservableObject {
    @Published var response: AnalyticsResponse?
    @Published var errorMessage: String?
    @Published var isLoading = false

    func refresh(days: Int) async {
        isLoading = true
        errorMessage = nil
        defer { isLoading = false }

        do {
            let data = try await CLIRunner.run(arguments: ["analytics", "--days", "\(days)", "--json"])
            let decoded = try JSONDecoder().decode(AnalyticsResponse.self, from: data)
            response = decoded
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}
