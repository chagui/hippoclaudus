import SwiftUI

// MARK: - Content View

struct ContentView: View {
    @ObservedObject var statusProvider: StatusProvider
    @ObservedObject var searchProvider: SearchProvider
    @ObservedObject var enrichmentProvider: GitEnrichmentProvider
    @ObservedObject var repoProvider: RepoProvider
    @ObservedObject var vaultTagProvider: VaultTagProvider
    @State private var expandedSessions: Set<String> = []

    private var isSearchActive: Bool {
        !searchProvider.query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    var body: some View {
        VStack(spacing: 0) {
            TopBar(query: $searchProvider.query, onSearch: searchProvider.search, onClear: searchProvider.clearSearch)
            Divider()

            if isSearchActive {
                SearchResultsDetail(searchProvider: searchProvider)
            } else {
                ScrollView {
                    VStack(spacing: 8) {
                        // Session cards
                        ForEach(statusProvider.activeSessions, id: \.id) { session in
                            SessionCard(
                                session: session,
                                enrichment: enrichmentProvider.enrichments[session.id],
                                isExpanded: expandedSessions.contains(session.id),
                                onToggle: { toggleSession(session.id) }
                            )
                        }

                        if statusProvider.activeSessions.isEmpty {
                            EmptyStateView(statusProvider: statusProvider)
                        }
                    }
                    .padding(.horizontal, 12)
                    .padding(.top, 8)
                    .padding(.bottom, 4)

                    ToolsSection(
                        statusProvider: statusProvider,
                        repoProvider: repoProvider,
                        vaultTagProvider: vaultTagProvider
                    )
                }
            }

            Divider()
            BottomBar(statusProvider: statusProvider)
        }
    }

    private func toggleSession(_ id: String) {
        withAnimation(.easeInOut(duration: 0.15)) {
            if expandedSessions.contains(id) {
                expandedSessions.remove(id)
            } else {
                expandedSessions.insert(id)
            }
        }
    }
}

// MARK: - Top Bar

struct TopBar: View {
    @Binding var query: String
    var onSearch: () -> Void
    var onClear: () -> Void

    var body: some View {
        HStack(spacing: 8) {
            SearchBar(query: $query, onSearch: onSearch, onClear: onClear)
            Text("Claude Pulse")
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
    }
}

// MARK: - Search Bar

struct SearchBar: View {
    @Binding var query: String
    var onSearch: () -> Void
    var onClear: () -> Void

    var body: some View {
        HStack(spacing: 6) {
            Image(systemName: "magnifyingglass")
                .foregroundStyle(.secondary)
                .font(.system(size: 12))

            TextField("Search vault...", text: $query)
                .textFieldStyle(.plain)
                .font(.system(size: 12))
                .onSubmit { onSearch() }

            if !query.isEmpty {
                Button(action: onClear) {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundStyle(.secondary)
                        .font(.system(size: 11))
                }
                .buttonStyle(.plain)
            }
        }
        .padding(6)
        .background(Color(nsColor: .controlBackgroundColor))
        .clipShape(RoundedRectangle(cornerRadius: 8))
    }
}

// MARK: - Session Card

struct SessionCard: View {
    let session: ActiveSessionResponse
    let enrichment: SessionEnrichment?
    let isExpanded: Bool
    let onToggle: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Header: status dot + name + model badge
            HStack(spacing: 8) {
                Circle()
                    .fill(session.isWaiting ? Color.yellow : Color.green)
                    .frame(width: 8, height: 8)

                Text(session.projectName)
                    .font(.system(size: 13, weight: .medium))
                    .lineLimit(1)

                if enrichment?.isWorktree == true {
                    Image(systemName: "arrow.triangle.branch")
                        .font(.system(size: 9))
                        .foregroundStyle(.orange)
                }

                Spacer()

                Text(session.modelLabel)
                    .font(.system(size: 10, weight: .medium))
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(Capsule().fill(Color(nsColor: .separatorColor).opacity(0.3)))
            }

            // Status + duration
            HStack(spacing: 4) {
                Text(session.isWaiting ? "Waiting for input" : "Working")
                Text("\u{00B7}").foregroundStyle(.tertiary)
                Text(session.duration)
            }
            .font(.system(size: 11))
            .foregroundStyle(.secondary)
            .padding(.top, 4)
            .padding(.leading, 16)

            // Key metrics (always visible)
            VStack(spacing: 0) {
                CardMetricRow(label: "Tokens", value: session.totalTokensFormatted)
                CardMetricRow(label: "Cost", value: session.estimatedCost)
            }
            .padding(.top, 6)
            .padding(.leading, 16)

            // Expanded details
            if isExpanded {
                VStack(spacing: 0) {
                    if enrichment?.isGitRepo == true {
                        CardMetricRow(label: "Branch", value: session.gitBranch)
                    }
                    CardMetricRow(label: "Turns", value: "\(session.exchangeCount)")
                    CardMetricRow(label: "Avg Turn", value: session.avgTurnDuration)
                    if let timeSplit = session.timeSplit {
                        CardMetricRow(label: "Time Split", value: timeSplit)
                    }
                    if let toolSummary = session.toolSummary {
                        CardMetricRow(label: "Tools", value: toolSummary)
                    }
                    if let files = session.filesTouchedCount, files > 0 {
                        CardMetricRow(label: "Files", value: "\(files) touched")
                    }
                }
                .padding(.leading, 16)

                if let prInfo = enrichment?.prInfo {
                    PRCard(prInfo: prInfo)
                        .padding(.top, 6)
                }
            }

            // Expand/collapse chevron
            Button(action: onToggle) {
                Image(systemName: isExpanded ? "chevron.up" : "chevron.down")
                    .font(.system(size: 10, weight: .medium))
                    .foregroundStyle(.tertiary)
                    .frame(maxWidth: .infinity)
                    .padding(.top, 6)
            }
            .buttonStyle(.plain)
        }
        .padding(12)
        .background(Color(nsColor: .controlBackgroundColor).opacity(0.5))
        .clipShape(RoundedRectangle(cornerRadius: 10))
    }
}

// MARK: - Card Metric Row

struct CardMetricRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack {
            Text(label)
                .font(.system(size: 11))
                .foregroundStyle(.secondary)
            Spacer()
            Text(value)
                .font(.system(size: 11, weight: .medium))
                .lineLimit(1)
        }
        .padding(.vertical, 2)
    }
}

// MARK: - PR Card

struct PRCard: View {
    let prInfo: PRInfo

    private var stateColor: Color {
        if prInfo.isDraft { return .secondary }
        switch prInfo.state {
        case "OPEN": return .green
        case "MERGED": return .purple
        case "CLOSED": return .red
        default: return .secondary
        }
    }

    private var stateLabel: String {
        if prInfo.isDraft { return "Draft" }
        switch prInfo.state {
        case "OPEN": return "Open"
        case "MERGED": return "Merged"
        case "CLOSED": return "Closed"
        default: return prInfo.state
        }
    }

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "arrow.triangle.pull")
                .font(.system(size: 12))
                .foregroundStyle(stateColor)

            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 4) {
                    Text("#\(prInfo.number)")
                        .font(.system(size: 11, weight: .semibold))
                    Text(stateLabel)
                        .font(.system(size: 9, weight: .medium))
                        .foregroundStyle(.white)
                        .padding(.horizontal, 5)
                        .padding(.vertical, 1)
                        .background(Capsule().fill(stateColor))
                }
                Text(prInfo.title)
                    .font(.system(size: 10))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer()

            Button(action: {
                if let url = URL(string: prInfo.url) {
                    NSWorkspace.shared.open(url)
                }
            }) {
                Image(systemName: "arrow.up.right.square")
                    .font(.system(size: 11))
                    .foregroundStyle(.secondary)
            }
            .buttonStyle(.plain)
            .help("Open PR in browser")
        }
        .padding(8)
        .background(Color(nsColor: .controlBackgroundColor).opacity(0.3))
        .clipShape(RoundedRectangle(cornerRadius: 8))
    }
}

// MARK: - Empty State View

struct EmptyStateView: View {
    @ObservedObject var statusProvider: StatusProvider

    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: "brain.head.profile")
                .font(.system(size: 24))
                .foregroundStyle(.tertiary)

            Text("No active sessions")
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(.secondary)

            VStack(spacing: 0) {
                CardMetricRow(label: "Last Sync", value: statusProvider.lastSyncAgo)
                CardMetricRow(label: "Vault Notes", value: "\(statusProvider.vaultNotes)")
                CardMetricRow(label: "Unprocessed", value: "\(statusProvider.unprocessed)")
            }
            .padding(.top, 4)

            if let errorMessage = statusProvider.errorMessage {
                Text(errorMessage)
                    .font(.system(size: 10))
                    .foregroundStyle(.red)
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity)
        .background(Color(nsColor: .controlBackgroundColor).opacity(0.5))
        .clipShape(RoundedRectangle(cornerRadius: 10))
    }
}

// MARK: - Tools Section

struct ToolsSection: View {
    @ObservedObject var statusProvider: StatusProvider
    @ObservedObject var repoProvider: RepoProvider
    @ObservedObject var vaultTagProvider: VaultTagProvider
    @State private var showRepos = false
    @State private var showTags = false

    var body: some View {
        VStack(spacing: 0) {
            // Section header
            HStack(spacing: 6) {
                Image(systemName: "ellipsis.circle.fill")
                    .font(.system(size: 11))
                    .foregroundStyle(.secondary)
                Text("Tools")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(.secondary)
                Spacer()
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 6)

            Divider().padding(.horizontal, 12)

            // Expandable sections
            ToolMenuItem(icon: "folder", label: "Repositories", count: repoProvider.repos.count, isExpanded: showRepos) {
                withAnimation(.easeInOut(duration: 0.15)) { showRepos.toggle() }
            }

            if showRepos {
                VStack(spacing: 0) {
                    ForEach(repoProvider.repos) { repo in
                        HStack(spacing: 6) {
                            Text(repo.projectName)
                                .font(.system(size: 11))
                                .lineLimit(1)
                            Spacer()
                            Text("\(repo.sessionCount)")
                                .font(.system(size: 10))
                                .foregroundStyle(.tertiary)
                        }
                        .padding(.horizontal, 36)
                        .padding(.vertical, 4)
                    }
                }
                .padding(.bottom, 2)
            }

            ToolMenuItem(icon: "tag", label: "Tags", count: vaultTagProvider.tags.count, isExpanded: showTags) {
                withAnimation(.easeInOut(duration: 0.15)) { showTags.toggle() }
            }

            if showTags {
                VStack(spacing: 0) {
                    ForEach(vaultTagProvider.tags) { tag in
                        HStack(spacing: 6) {
                            Text(tag.name)
                                .font(.system(size: 11))
                                .lineLimit(1)
                            Spacer()
                            Text("\(tag.documentCount)")
                                .font(.system(size: 10))
                                .foregroundStyle(.tertiary)
                        }
                        .padding(.horizontal, 36)
                        .padding(.vertical, 4)
                        .contentShape(Rectangle())
                        .onTapGesture {
                            if let doc = tag.documents.first {
                                vaultTagProvider.openInObsidian(document: doc)
                            }
                        }
                    }
                }
                .padding(.bottom, 2)
            }

            // Action items
            ToolMenuItem(icon: "arrow.triangle.2.circlepath", label: statusProvider.isSyncing ? "Syncing..." : "Sync Now") {
                Task { await statusProvider.syncNow() }
            }
            .disabled(statusProvider.isSyncing)

            ToolMenuItem(icon: "folder.badge.gearshape", label: "Open Vault") {
                if let url = URL(string: "obsidian://open?vault=Claude") {
                    NSWorkspace.shared.open(url)
                }
            }

            ToolMenuItem(icon: "power", label: "Quit Claude Pulse") {
                NSApplication.shared.terminate(nil)
            }
        }
    }
}

// MARK: - Tool Menu Item

struct ToolMenuItem: View {
    let icon: String
    let label: String
    var count: Int? = nil
    var isExpanded: Bool = false
    let action: () -> Void

    @State private var isHovered = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) {
                Image(systemName: icon)
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
                    .frame(width: 16)

                Text(label)
                    .font(.system(size: 13))

                Spacer()

                if let count {
                    Text("\(count)")
                        .font(.system(size: 11))
                        .foregroundStyle(.tertiary)
                    Image(systemName: isExpanded ? "chevron.down" : "chevron.right")
                        .font(.system(size: 9, weight: .medium))
                        .foregroundStyle(.tertiary)
                }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 7)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(isHovered ? Color(nsColor: .controlBackgroundColor).opacity(0.6) : Color.clear)
            .clipShape(RoundedRectangle(cornerRadius: 6))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering in isHovered = hovering }
        .padding(.horizontal, 4)
    }
}

// MARK: - Bottom Bar

struct BottomBar: View {
    @ObservedObject var statusProvider: StatusProvider

    var body: some View {
        HStack(spacing: 6) {
            if statusProvider.isSyncing {
                ProgressView()
                    .controlSize(.mini)
                Text("Syncing...")
                    .font(.system(size: 10))
                    .foregroundStyle(.secondary)
            } else {
                Text(statusText)
                    .font(.system(size: 10))
                    .foregroundStyle(.secondary)
            }

            Spacer()

            HStack(spacing: 6) {
                Button(action: { Task { await statusProvider.syncNow() } }) {
                    Image(systemName: "arrow.triangle.2.circlepath")
                        .font(.system(size: 11))
                }
                .buttonStyle(.plain)
                .disabled(statusProvider.isSyncing)
                .help("Sync Now")

                Button(action: {
                    if let url = URL(string: "obsidian://open?vault=Claude") {
                        NSWorkspace.shared.open(url)
                    }
                }) {
                    Image(systemName: "folder")
                        .font(.system(size: 11))
                }
                .buttonStyle(.plain)
                .help("Open Vault")
            }
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
    }

    private var statusText: String {
        let count = statusProvider.activeSessionCount
        let sessions = count == 0 ? "No sessions" : "\(count) active"
        return "\(sessions) \u{00B7} synced \(statusProvider.lastSyncAgo)"
    }
}

// MARK: - Search Results Detail

struct SearchResultsDetail: View {
    @ObservedObject var searchProvider: SearchProvider

    var body: some View {
        if searchProvider.isSearching {
            VStack {
                Spacer()
                ProgressView("Searching...")
                    .font(.caption)
                Spacer()
            }
        } else if searchProvider.results.isEmpty {
            VStack {
                Spacer()
                Text("No results found")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Spacer()
            }
        } else {
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 4) {
                    ForEach(searchProvider.results) { result in
                        SearchResultRow(result: result)
                            .onTapGesture {
                                searchProvider.openInObsidian(result: result)
                            }
                    }
                }
                .padding(10)
            }
        }
    }
}

struct SearchResultRow: View {
    let result: SearchResult

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(result.title)
                .font(.system(size: 13, weight: .semibold))
                .lineLimit(1)

            if !result.tags.isEmpty {
                HStack(spacing: 4) {
                    ForEach(result.tags.prefix(5), id: \.self) { tag in
                        Text(tag)
                            .font(.system(size: 10))
                            .foregroundStyle(.secondary)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(Color(nsColor: .separatorColor).opacity(0.3))
                            .clipShape(Capsule())
                    }
                }
            }

            ForEach(Array(result.excerpts.enumerated()), id: \.offset) { _, excerpt in
                Text(excerpt)
                    .font(.system(size: 11))
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(8)
        .background(Color(nsColor: .controlBackgroundColor).opacity(0.5))
        .clipShape(RoundedRectangle(cornerRadius: 6))
        .contentShape(Rectangle())
    }
}
