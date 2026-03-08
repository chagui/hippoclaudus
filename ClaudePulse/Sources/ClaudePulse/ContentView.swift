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
                    VStack(spacing: 0) {
                        if !statusProvider.activeSessions.isEmpty {
                            SectionHeader(title: "Sessions")

                            ForEach(statusProvider.activeSessions, id: \.id) { session in
                                SessionCard(
                                    session: session,
                                    enrichment: enrichmentProvider.enrichments[session.id],
                                    isExpanded: expandedSessions.contains(session.id),
                                    onToggle: { toggleSession(session.id) }
                                )
                                if session.id != statusProvider.activeSessions.last?.id {
                                    Divider().padding(.horizontal, 12)
                                }
                            }
                        } else {
                            EmptyStateView(statusProvider: statusProvider)
                        }
                    }

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
    @State private var showSearch = false

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "brain.head.profile")
                .font(.system(size: 13))
                .foregroundStyle(.secondary)

            if showSearch {
                TextField("Search vault...", text: $query)
                    .textFieldStyle(.plain)
                    .font(.system(size: 12))
                    .onSubmit { onSearch() }
            } else {
                Spacer()
                Text("Claude Pulse")
                    .font(.system(size: 13, weight: .semibold))
                Spacer()
            }

            Button(action: {
                withAnimation(.easeInOut(duration: 0.15)) {
                    showSearch.toggle()
                    if !showSearch { onClear() }
                }
            }) {
                Image(systemName: showSearch ? "xmark" : "magnifyingglass")
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
            }
            .buttonStyle(.plain)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }
}

// MARK: - Section Header

struct SectionHeader: View {
    let title: String
    var actionIcon: String? = nil
    var action: (() -> Void)? = nil

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Spacer()
                Text(title.uppercased())
                    .font(.system(size: 10, weight: .medium))
                    .foregroundStyle(.tertiary)
                    .tracking(0.5)
                if let actionIcon, let action {
                    Button(action: action) {
                        Image(systemName: actionIcon)
                            .font(.system(size: 10))
                            .foregroundStyle(.tertiary)
                    }
                    .buttonStyle(.plain)
                }
                Spacer()
            }
            .padding(.vertical, 6)

            Divider().padding(.horizontal, 12)
        }
    }
}

// MARK: - Detail Row

struct DetailRow: View {
    let label: String
    let value: String
    var dotColor: Color? = nil

    var body: some View {
        HStack(spacing: 6) {
            if let dotColor {
                Circle()
                    .fill(dotColor)
                    .frame(width: 6, height: 6)
            }
            Text(label)
                .font(.system(size: 11))
                .foregroundStyle(.secondary)
            Spacer()
            Text(value)
                .font(.system(size: 11, weight: .medium))
                .lineLimit(1)
        }
        .padding(.vertical, 2.5)
        .padding(.horizontal, 12)
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
            // Header: status dot + project name + model badge + chevron
            Button(action: onToggle) {
                HStack(spacing: 8) {
                    Circle()
                        .fill(session.isWaiting ? Color.yellow : Color.green)
                        .frame(width: 8, height: 8)

                    Text(session.projectName)
                        .font(.system(size: 12, weight: .medium))
                        .lineLimit(1)

                    if enrichment?.isWorktree == true {
                        Image(systemName: "arrow.triangle.branch")
                            .font(.system(size: 9))
                            .foregroundStyle(.orange)
                        if let label = enrichment?.worktreeLabel {
                            Text(label)
                                .font(.system(size: 10, weight: .medium))
                                .foregroundStyle(.orange)
                                .lineLimit(1)
                        }
                    }

                    Spacer()

                    Text(session.modelLabel)
                        .font(.system(size: 10, weight: .medium))
                        .foregroundStyle(.secondary)
                        .padding(.horizontal, 5)
                        .padding(.vertical, 1)
                        .background(Capsule().fill(Color(nsColor: .separatorColor).opacity(0.3)))

                    Image(systemName: isExpanded ? "chevron.up" : "chevron.down")
                        .font(.system(size: 9, weight: .medium))
                        .foregroundStyle(.tertiary)
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .padding(.horizontal, 12)
            .padding(.top, 8)
            .padding(.bottom, 4)

            // Status + duration
            HStack(spacing: 4) {
                Text(session.isWaiting ? "Waiting for input" : "Working")
                Text("\u{00B7}").foregroundStyle(.tertiary)
                Text(session.duration)
            }
            .font(.system(size: 11))
            .foregroundStyle(.secondary)
            .padding(.horizontal, 12)
            .padding(.leading, 16)
            .padding(.bottom, 4)

            // Key metrics (always visible)
            DetailRow(label: "Tokens", value: session.totalTokensFormatted)
            DetailRow(label: "Cost", value: session.estimatedCost)

            // Expanded details
            if isExpanded {
                if enrichment?.isGitRepo == true {
                    DetailRow(label: "Branch", value: session.gitBranch)
                }
                if let wtLabel = enrichment?.worktreeLabel {
                    DetailRow(label: "Worktree", value: wtLabel)
                }
                DetailRow(label: "Turns", value: "\(session.exchangeCount)")
                DetailRow(label: "Avg Turn", value: session.avgTurnDuration)
                if let timeSplit = session.timeSplit {
                    DetailRow(label: "Time Split", value: timeSplit)
                }
                if let toolSummary = session.toolSummary {
                    DetailRow(label: "Tools", value: toolSummary)
                }
                if let files = session.filesTouchedCount, files > 0 {
                    DetailRow(label: "Files", value: "\(files) touched")
                }

                if let prInfo = enrichment?.prInfo {
                    PRCard(prInfo: prInfo)
                        .padding(.horizontal, 12)
                        .padding(.top, 4)
                }
            }
        }
        .padding(.bottom, 6)
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
                .font(.system(size: 11))
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
        .padding(.vertical, 6)
    }
}

// MARK: - Empty State View

struct EmptyStateView: View {
    @ObservedObject var statusProvider: StatusProvider

    var body: some View {
        VStack(spacing: 6) {
            Image(systemName: "brain.head.profile")
                .font(.system(size: 24))
                .foregroundStyle(.tertiary)
                .padding(.top, 16)

            Text("No active sessions")
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(.secondary)
                .padding(.bottom, 4)

            DetailRow(label: "Last Sync", value: statusProvider.lastSyncAgo)
            DetailRow(label: "Vault Notes", value: "\(statusProvider.vaultNotes)")
            DetailRow(label: "Unprocessed", value: "\(statusProvider.unprocessed)")

            if let errorMessage = statusProvider.errorMessage {
                Text(errorMessage)
                    .font(.system(size: 10))
                    .foregroundStyle(.red)
                    .padding(.horizontal, 12)
            }
        }
        .padding(.bottom, 8)
        .frame(maxWidth: .infinity)
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
            SectionHeader(title: "Tools")

            ToolMenuItem(icon: "folder", label: "Repositories", count: repoProvider.repos.count, isExpanded: showRepos) {
                withAnimation(.easeInOut(duration: 0.15)) { showRepos.toggle() }
            }

            if showRepos {
                VStack(spacing: 0) {
                    ForEach(repoProvider.repos) { repo in
                        if repo.worktreeCount > 0 {
                            DetailRow(label: repo.projectName, value: "\(repo.sessionCount) sessions (\(repo.worktreeCount) worktrees)")
                        } else {
                            DetailRow(label: repo.projectName, value: "\(repo.sessionCount)")
                        }
                    }
                }
            }

            ToolMenuItem(icon: "tag", label: "Tags", count: vaultTagProvider.tags.count, isExpanded: showTags) {
                withAnimation(.easeInOut(duration: 0.15)) { showTags.toggle() }
            }

            if showTags {
                VStack(spacing: 0) {
                    ForEach(vaultTagProvider.tags) { tag in
                        DetailRow(label: tag.name, value: "\(tag.documentCount)")
                            .contentShape(Rectangle())
                            .onTapGesture {
                                if let doc = tag.documents.first {
                                    vaultTagProvider.openInObsidian(document: doc)
                                }
                            }
                    }
                }
            }

            Divider().padding(.horizontal, 12).padding(.vertical, 4)

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
                    .font(.system(size: 11))
                    .foregroundStyle(.secondary)
                    .frame(width: 16)

                Text(label)
                    .font(.system(size: 12))

                Spacer()

                if let count {
                    Text("\(count)")
                        .font(.system(size: 10))
                        .foregroundStyle(.tertiary)
                    Image(systemName: isExpanded ? "chevron.down" : "chevron.right")
                        .font(.system(size: 9, weight: .medium))
                        .foregroundStyle(.tertiary)
                }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(isHovered ? Color(nsColor: .controlBackgroundColor).opacity(0.6) : Color.clear)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering in isHovered = hovering }
    }
}

// MARK: - Bottom Bar

struct BottomBar: View {
    @ObservedObject var statusProvider: StatusProvider

    var body: some View {
        HStack(spacing: 4) {
            if statusProvider.isSyncing {
                ProgressView()
                    .controlSize(.mini)
                Text("Syncing...")
                    .font(.system(size: 10))
                    .foregroundStyle(.secondary)
            } else {
                if statusProvider.errorMessage != nil {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .font(.system(size: 10))
                        .foregroundStyle(.red)
                        .help(statusProvider.errorMessage!)
                }
                Text(statusText)
                    .font(.system(size: 10))
                    .foregroundStyle(.tertiary)
            }

            Spacer()

            HStack(spacing: 4) {
                if statusProvider.isSyncing {
                    Button(action: { statusProvider.cancelSync() }) {
                        Image(systemName: "xmark.circle")
                            .font(.system(size: 10))
                    }
                    .buttonStyle(.plain)
                    .help("Cancel Sync")
                } else {
                    Button(action: { Task { await statusProvider.syncNow() } }) {
                        Image(systemName: "arrow.triangle.2.circlepath")
                            .font(.system(size: 10))
                    }
                    .buttonStyle(.plain)
                    .help("Sync Now")
                }

                Button(action: {
                    if let url = URL(string: "obsidian://open?vault=Claude") {
                        NSWorkspace.shared.open(url)
                    }
                }) {
                    Image(systemName: "folder")
                        .font(.system(size: 10))
                }
                .buttonStyle(.plain)
                .help("Open Vault")
            }
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 5)
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
                LazyVStack(alignment: .leading, spacing: 0) {
                    ForEach(searchProvider.results) { result in
                        SearchResultRow(result: result)
                            .onTapGesture {
                                searchProvider.openInObsidian(result: result)
                            }
                        Divider().padding(.horizontal, 12)
                    }
                }
            }
        }
    }
}

struct SearchResultRow: View {
    let result: SearchResult

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(result.title)
                .font(.system(size: 12, weight: .semibold))
                .lineLimit(1)

            if !result.tags.isEmpty {
                HStack(spacing: 4) {
                    ForEach(result.tags.prefix(5), id: \.self) { tag in
                        Text(tag)
                            .font(.system(size: 10))
                            .foregroundStyle(.secondary)
                            .padding(.horizontal, 5)
                            .padding(.vertical, 1)
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
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .contentShape(Rectangle())
    }
}
