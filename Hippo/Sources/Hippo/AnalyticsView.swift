import Charts
import SwiftUI

// MARK: - Formatting helpers

private func formatCost(_ v: Double) -> String {
    if v <= 0 { return "$0" }
    if v < 0.01 { return "<$0.01" }
    if v >= 1000 { return String(format: "$%.0f", v) }
    return String(format: "$%.2f", v)
}

private func formatTokens(_ v: Double) -> String {
    if v >= 1_000_000 { return String(format: "%.1fM", v / 1_000_000) }
    if v >= 1000 { return String(format: "%.0fK", v / 1000) }
    return String(format: "%.0f", v)
}

private func formatDurationMs(_ v: Double) -> String {
    let totalSecs = Int(v / 1000)
    let d = totalSecs / 86400
    let h = (totalSecs % 86400) / 3600
    let m = (totalSecs % 3600) / 60
    let s = totalSecs % 60
    if d > 0 { return "\(d)d \(h)h" }
    if h > 0 { return "\(h)h \(m)m" }
    if m > 0 { return "\(m)m \(s)s" }
    return "\(s)s"
}

private func formatPct(_ v: Double) -> String {
    String(format: "%.0f%%", v)
}

private func formatInt(_ v: Double) -> String {
    String(format: "%.0f", v)
}

// MARK: - Root view

struct AnalyticsView: View {
    @StateObject private var provider = AnalyticsProvider()
    @State private var days: Int = 30

    private let dayOptions: [Int] = [7, 30, 90]

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            Divider()

            if provider.isLoading, provider.response == nil {
                loadingState
            } else if let err = provider.errorMessage, provider.response == nil {
                errorState(err)
            } else if let response = provider.response {
                ScrollView {
                    VStack(alignment: .leading, spacing: 18) {
                        totalsRow(response)
                        Divider()
                        dailyChart(response)
                        Divider()
                        modelChart(response)
                        Divider()
                        percentileTable(response)
                    }
                    .padding(16)
                }
            } else {
                emptyState
            }
        }
        .frame(minWidth: 640, idealWidth: 720, minHeight: 600, idealHeight: 780)
        .task {
            await provider.refresh(days: days)
        }
    }

    // MARK: - Header

    private var header: some View {
        HStack(spacing: 12) {
            Text("Session Analytics")
                .font(.system(size: 15, weight: .semibold))

            Spacer()

            Picker("Window", selection: $days) {
                ForEach(dayOptions, id: \.self) { d in
                    Text("\(d)d").tag(d)
                }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .frame(width: 180)
            .onChange(of: days) { _, newValue in
                Task { await provider.refresh(days: newValue) }
            }

            Button {
                Task { await provider.refresh(days: days) }
            } label: {
                Image(systemName: "arrow.clockwise")
                    .font(.system(size: 12))
            }
            .buttonStyle(.borderless)
            .disabled(provider.isLoading)
        }
        .padding(12)
    }

    // MARK: - States

    private var loadingState: some View {
        VStack {
            Spacer()
            ProgressView("Loading…")
            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func errorState(_ message: String) -> some View {
        VStack(spacing: 12) {
            Spacer()
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundStyle(.red)
                .font(.system(size: 24))
            Text("Couldn't load analytics")
                .font(.system(size: 13, weight: .medium))
            Text(message)
                .font(.system(size: 11))
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 400)
            Button("Retry") {
                Task { await provider.refresh(days: days) }
            }
            .controlSize(.small)
            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var emptyState: some View {
        VStack {
            Spacer()
            Text("No sessions in window")
                .foregroundStyle(.secondary)
            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    // MARK: - Totals row

    private func totalsRow(_ r: AnalyticsResponse) -> some View {
        HStack(spacing: 16) {
            TotalTile(label: "Sessions", value: "\(r.sessionCount)")
            TotalTile(label: "Total cost", value: formatCost(r.totals.costUsd))
            TotalTile(label: "Total tokens", value: formatTokens(Double(r.totals.totalTokens)))
            TotalTile(label: "Total duration", value: formatDurationMs(Double(r.totals.totalDurationMs)))
        }
    }

    // MARK: - Percentile table

    private struct MetricRow {
        let key: String
        let label: String
        let formatter: (Double) -> String
    }

    private var metricRows: [MetricRow] {
        [
            MetricRow(key: "cost_usd", label: "Cost", formatter: formatCost),
            MetricRow(key: "total_tokens", label: "Tokens", formatter: formatTokens),
            MetricRow(key: "turn_count", label: "Turns", formatter: formatInt),
            MetricRow(key: "avg_turn_duration_ms", label: "Avg turn", formatter: formatDurationMs),
            MetricRow(key: "total_duration_ms", label: "Duration", formatter: formatDurationMs),
            MetricRow(key: "agent_time_pct", label: "Agent %", formatter: formatPct),
            MetricRow(key: "write_count", label: "Writes", formatter: formatInt),
            MetricRow(key: "edit_count", label: "Edits", formatter: formatInt),
            MetricRow(key: "bash_count", label: "Bash", formatter: formatInt),
            MetricRow(key: "files_touched_count", label: "Files", formatter: formatInt),
        ]
    }

    private func percentileTable(_ r: AnalyticsResponse) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            SectionLabel("Distributions")

            Grid(alignment: .leading, horizontalSpacing: 12, verticalSpacing: 4) {
                GridRow {
                    Text("Metric").gridColumnAlignment(.leading)
                    Text("p50").gridColumnAlignment(.trailing)
                    Text("p90").gridColumnAlignment(.trailing)
                    Text("p95").gridColumnAlignment(.trailing)
                    Text("p99").gridColumnAlignment(.trailing)
                    Text("max").gridColumnAlignment(.trailing)
                }
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(.secondary)

                Divider().gridCellColumns(6)

                ForEach(metricRows, id: \.key) { row in
                    if let d = r.distributions[row.key] {
                        GridRow {
                            Text(row.label).foregroundStyle(.secondary)
                            Text(row.formatter(d.p50)).monospacedDigit()
                            Text(row.formatter(d.p90)).monospacedDigit()
                            Text(row.formatter(d.p95)).monospacedDigit()
                            Text(row.formatter(d.p99)).monospacedDigit()
                            Text(row.formatter(d.max)).monospacedDigit()
                        }
                        .font(.system(size: 11))
                    }
                }
            }
            .padding(12)
            .background(Color(nsColor: .controlBackgroundColor).opacity(0.4))
            .clipShape(RoundedRectangle(cornerRadius: 6))
        }
    }

    // MARK: - Daily time-series chart

    @State private var dailyMetric: DailyMetric = .cost
    @State private var hoveredDate: Date?

    private enum DailyMetric: String, CaseIterable, Identifiable {
        case cost = "Cost"
        case tokens = "Tokens"
        var id: String {
            rawValue
        }
    }

    private func dailyChart(_ r: AnalyticsResponse) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                SectionLabel("Daily")
                Spacer()
                Picker("Metric", selection: $dailyMetric) {
                    ForEach(DailyMetric.allCases) { m in
                        Text(m.rawValue).tag(m)
                    }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .frame(width: 160)
            }

            // Stack order: Opus at bottom, Sonnet middle, Haiku top, Other above.
            let stackOrder = modelStackOrder(for: r.daily)
            Chart(r.daily) { bucket in
                BarMark(
                    x: .value("Date", isoDate(bucket.date) ?? Date()),
                    y: .value(dailyMetric.rawValue, dailyValue(bucket)),
                )
                .foregroundStyle(by: .value("Model", bucket.model))
                .position(by: .value("Model", bucket.model), axis: .vertical)
            }
            .chartForegroundStyleScale(
                domain: stackOrder,
                range: stackOrder.map(colorForModel),
            )
            .chartLegend(position: .top, alignment: .trailing, spacing: 4)
            .chartXSelection(value: $hoveredDate)
            .chartXScale(domain: chartDomain(r.window))
            .chartOverlay { proxy in
                GeometryReader { geo in
                    if let hovered = hoveredDate,
                       let snapped = snapToBucketDate(hovered, in: r.daily),
                       let xPos = proxy.position(forX: snapped)
                    {
                        let rect = geo.frame(in: .local)
                        let sameDayRows = r.daily.filter { isoDate($0.date).map { Calendar.current.isDate($0, inSameDayAs: snapped) } ?? false }
                        ZStack(alignment: .topLeading) {
                            Rectangle()
                                .fill(Color.secondary.opacity(0.25))
                                .frame(width: 1, height: rect.height)
                                .offset(x: xPos, y: 0)
                            dayTooltip(for: snapped, rows: sameDayRows)
                                .fixedSize()
                                .offset(tooltipOffset(xPos: xPos, rect: rect))
                        }
                    }
                }
            }
            .chartYAxis {
                AxisMarks(position: .leading) { value in
                    AxisGridLine()
                    AxisValueLabel {
                        if let v = value.as(Double.self) {
                            Text(dailyMetric == .cost ? formatCost(v) : formatTokens(v))
                                .font(.system(size: 9))
                        }
                    }
                }
            }
            .chartXAxis {
                AxisMarks(values: .automatic(desiredCount: 6)) { _ in
                    AxisGridLine()
                    AxisValueLabel(format: .dateTime.month(.abbreviated).day(), centered: true)
                }
            }
            .frame(height: 200)
        }
    }

    private func chartDomain(_ window: AnalyticsWindow) -> ClosedRange<Date> {
        let start = isoDate(window.start) ?? Date().addingTimeInterval(-30 * 86400)
        let end = isoDate(window.end) ?? Date()
        // Pad the range by half a day on each side so edge bars aren't clipped.
        let half: TimeInterval = 43200
        return start.addingTimeInterval(-half) ... end.addingTimeInterval(half)
    }

    /// Snap a continuous hover x-value back to a bucket date we actually have data for.
    private func snapToBucketDate(_ date: Date, in buckets: [AnalyticsDailyBucket]) -> Date? {
        let unique = Set(buckets.compactMap { isoDate($0.date) })
        return unique.min { a, b in
            a.timeIntervalSince(date).magnitude < b.timeIntervalSince(date).magnitude
        }
    }

    private func modelStackOrder(for buckets: [AnalyticsDailyBucket]) -> [String] {
        let preferred = ["opus", "sonnet", "haiku", "other"]
        let present = Set(buckets.map(\.model))
        return preferred.filter(present.contains) + present.subtracting(preferred).sorted()
    }

    /// Keep the tooltip from overflowing the chart: shift left as we approach the right edge.
    private func tooltipOffset(xPos: CGFloat, rect: CGRect) -> CGSize {
        let tooltipWidth: CGFloat = 200
        let padding: CGFloat = 8
        let desired = xPos + padding
        let maxX = rect.width - tooltipWidth - padding
        let x = min(desired, maxX)
        return CGSize(width: max(0, x), height: padding)
    }

    @ViewBuilder
    private func dayTooltip(for date: Date, rows: [AnalyticsDailyBucket]) -> some View {
        let totalValue = rows.reduce(0.0) { $0 + dailyValue($1) }
        let totalSessions = rows.reduce(0) { $0 + $1.sessionCount }
        VStack(alignment: .leading, spacing: 3) {
            Text(formatTooltipDate(date))
                .font(.system(size: 10, weight: .semibold))
            HStack(spacing: 6) {
                Text(dailyMetric == .cost ? formatCost(totalValue) : formatTokens(totalValue))
                    .font(.system(size: 11, weight: .medium))
                    .monospacedDigit()
                Text("·")
                    .foregroundStyle(.tertiary)
                Text("\(totalSessions) \(totalSessions == 1 ? "session" : "sessions")")
                    .font(.system(size: 10))
                    .foregroundStyle(.secondary)
            }
            if rows.count > 1 {
                Divider()
                    .padding(.vertical, 1)
                ForEach(rows.sorted { dailyValue($0) > dailyValue($1) }) { row in
                    HStack(spacing: 6) {
                        Circle()
                            .fill(colorForModel(row.model))
                            .frame(width: 6, height: 6)
                        Text(row.model.capitalized)
                            .font(.system(size: 10))
                            .foregroundStyle(.secondary)
                        Spacer(minLength: 8)
                        Text(dailyMetric == .cost
                            ? formatCost(row.costUsd)
                            : formatTokens(Double(row.totalTokens)))
                            .font(.system(size: 10))
                            .monospacedDigit()
                    }
                }
            }
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 5)
        .frame(maxWidth: 200, alignment: .leading)
        .background(Color(nsColor: .controlBackgroundColor))
        .overlay(
            RoundedRectangle(cornerRadius: 4)
                .stroke(Color.secondary.opacity(0.3), lineWidth: 0.5),
        )
        .clipShape(RoundedRectangle(cornerRadius: 4))
        .shadow(color: .black.opacity(0.15), radius: 3, y: 1)
    }

    private func dailyValue(_ bucket: AnalyticsDailyBucket) -> Double {
        switch dailyMetric {
        case .cost: bucket.costUsd
        case .tokens: Double(bucket.totalTokens)
        }
    }

    private func isoDate(_ str: String) -> Date? {
        let f = DateFormatter()
        f.dateFormat = "yyyy-MM-dd"
        f.timeZone = TimeZone.current
        return f.date(from: str)
    }

    /// "Fri, Apr 19, 2026" — weekday abbreviation prefixed to the abbreviated date.
    private func formatTooltipDate(_ date: Date) -> String {
        date.formatted(
            Date.FormatStyle()
                .weekday(.abbreviated)
                .month(.abbreviated)
                .day()
                .year(),
        )
    }

    // MARK: - By-model chart

    private func modelChart(_ r: AnalyticsResponse) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            SectionLabel("By model")

            if r.byModel.isEmpty {
                Text("No data").font(.system(size: 11)).foregroundStyle(.secondary)
            } else {
                Chart(r.byModel) { bucket in
                    BarMark(
                        x: .value("Cost", bucket.costUsd),
                        y: .value("Model", bucket.model),
                    )
                    .foregroundStyle(colorForModel(bucket.model))
                    .annotation(position: .trailing, alignment: .leading) {
                        Text("\(bucket.sessionCount) · \(formatCost(bucket.costUsd))")
                            .font(.system(size: 10))
                            .foregroundStyle(.secondary)
                    }
                }
                .chartXAxis {
                    AxisMarks { value in
                        AxisGridLine()
                        AxisValueLabel {
                            if let v = value.as(Double.self) {
                                Text(formatCost(v)).font(.system(size: 9))
                            }
                        }
                    }
                }
                .frame(height: CGFloat(max(r.byModel.count * 36, 80)))
            }
        }
    }

    private func colorForModel(_ name: String) -> Color {
        switch name {
        case "opus": .purple
        case "sonnet": .blue
        case "haiku": .orange
        default: .gray
        }
    }
}

// MARK: - Small building blocks

private struct SectionLabel: View {
    let text: String
    init(_ text: String) {
        self.text = text
    }

    var body: some View {
        Text(text)
            .font(.system(size: 13, weight: .bold))
            .foregroundStyle(.primary)
    }
}

private struct TotalTile: View {
    let label: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label.uppercased())
                .font(.system(size: 9, weight: .semibold))
                .foregroundStyle(.tertiary)
                .tracking(0.4)
            Text(value)
                .font(.system(size: 18, weight: .semibold))
                .monospacedDigit()
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(10)
        .background(Color(nsColor: .controlBackgroundColor).opacity(0.4))
        .clipShape(RoundedRectangle(cornerRadius: 6))
    }
}

// MARK: - Window controller

@MainActor
enum AnalyticsWindowController {
    private static var window: NSWindow?

    static func show() {
        if let existing = window, existing.isVisible {
            existing.makeKeyAndOrderFront(nil)
            NSApp.activate(ignoringOtherApps: true)
            return
        }

        let hostingView = NSHostingView(rootView: AnalyticsView())

        let win = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 720, height: 780),
            styleMask: [.titled, .closable, .resizable],
            backing: .buffered,
            defer: false,
        )
        win.contentView = hostingView
        win.title = "Session Analytics"
        win.center()
        win.isReleasedWhenClosed = false
        win.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)

        window = win
    }
}
