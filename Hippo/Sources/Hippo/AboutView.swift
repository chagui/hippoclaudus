import SwiftUI

struct AboutView: View {
    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "brain.head.profile")
                .font(.system(size: 48))
                .foregroundStyle(.secondary)

            Text("Hippoclaudus")
                .font(.system(size: 18, weight: .bold))

            Text("v0.1.0")
                .font(.system(size: 12))
                .foregroundStyle(.secondary)

            Text("A macOS companion for Claude Code.\nMonitors sessions, extracts knowledge,\nand syncs to your Obsidian vault.")
                .font(.system(size: 11))
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .lineSpacing(2)

            Link("GitHub", destination: URL(string: "https://github.com/chagui/hippoclaudus")!)
                .font(.system(size: 11))

            Text("\u{00A9} 2025\u{2013}2026 chagui")
                .font(.system(size: 10))
                .foregroundStyle(.tertiary)
        }
        .padding(24)
        .frame(width: 280)
    }
}

/// Manages the standalone About window so only one instance exists at a time.
@MainActor
enum AboutWindowController {
    private static var window: NSWindow?

    static func show() {
        if let existing = window, existing.isVisible {
            existing.makeKeyAndOrderFront(nil)
            NSApp.activate(ignoringOtherApps: true)
            return
        }

        let hostingView = NSHostingView(rootView: AboutView())
        hostingView.setFrameSize(hostingView.fittingSize)

        let win = NSWindow(
            contentRect: NSRect(origin: .zero, size: hostingView.fittingSize),
            styleMask: [.titled, .closable],
            backing: .buffered,
            defer: false,
        )
        win.contentView = hostingView
        win.title = "About Hippoclaudus"
        win.center()
        win.isReleasedWhenClosed = false
        win.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)

        window = win
    }
}
