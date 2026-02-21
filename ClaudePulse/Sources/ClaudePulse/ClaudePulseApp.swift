import AppKit
import SwiftUI

struct ClaudePulseApp: App {
    @StateObject private var statusProvider: StatusProvider
    @StateObject private var searchProvider = SearchProvider()
    @StateObject private var enrichmentProvider: GitEnrichmentProvider
    @StateObject private var repoProvider: RepoProvider
    @StateObject private var vaultTagProvider: VaultTagProvider

    init() {
        let enrichment = GitEnrichmentProvider()
        let repo = RepoProvider()
        let tags = VaultTagProvider()
        let status = StatusProvider(
            enrichmentProvider: enrichment,
            repoProvider: repo,
            vaultTagProvider: tags
        )
        _statusProvider = StateObject(wrappedValue: status)
        _enrichmentProvider = StateObject(wrappedValue: enrichment)
        _repoProvider = StateObject(wrappedValue: repo)
        _vaultTagProvider = StateObject(wrappedValue: tags)
    }

    var body: some Scene {
        MenuBarExtra {
            ContentView(
                statusProvider: statusProvider,
                searchProvider: searchProvider,
                enrichmentProvider: enrichmentProvider,
                repoProvider: repoProvider,
                vaultTagProvider: vaultTagProvider
            )
                .frame(width: 380, height: MenuBarLayout.panelHeight)
                .onAppear {
                    Task {
                        await repoProvider.refresh()
                        await vaultTagProvider.refresh()
                    }
                }
        } label: {
            Image(nsImage: MenuBarIcon.render(
                hasActive: statusProvider.activeSessionCount > 0,
                anyWaiting: statusProvider.anySessionWaiting
            ))
        }
        .menuBarExtraStyle(.window)
    }
}

/// Computes the panel height based on the screen containing the menu bar.
///
/// On large screens the panel uses up to 70% of the visible height (menu bar excluded),
/// capped at 900pt. On small screens it falls back to a 520pt minimum so the
/// layout never gets too cramped.
enum MenuBarLayout {
    static var panelHeight: CGFloat {
        // The menu bar lives on the screen with the key window, or the main screen
        let screen = NSScreen.main ?? NSScreen.screens.first
        guard let screen else { return 520 }
        let usable = screen.visibleFrame.height  // excludes menu bar + dock
        let desired = usable * 0.70
        return min(max(desired, 520), 900)
    }
}

/// Renders the menu bar icon as an NSImage with an optional colored status dot.
///
/// - No active sessions: template brain icon (adapts to light/dark automatically)
/// - Active, all working: brain icon + green dot
/// - Active, some waiting: brain icon + yellow dot
enum MenuBarIcon {
    static func render(hasActive: Bool, anyWaiting: Bool) -> NSImage {
        let symbolConfig = NSImage.SymbolConfiguration(pointSize: 16, weight: .regular)

        guard hasActive else {
            // No active sessions — return a template image (native menu bar rendering)
            guard let baseImage = NSImage(
                systemSymbolName: "brain.head.profile",
                accessibilityDescription: "Claude Pulse"
            ), let image = baseImage.withSymbolConfiguration(symbolConfig) else {
                // Fallback: simple text-based image if symbol is unavailable
                let fallback = NSImage(size: NSSize(width: 18, height: 18))
                fallback.isTemplate = true
                return fallback
            }
            image.isTemplate = true
            return image
        }

        // Active sessions — composite brain + colored status dot
        guard let baseBrain = NSImage(
            systemSymbolName: "brain.head.profile.fill",
            accessibilityDescription: "Claude Pulse"
        ), let brainImage = baseBrain.withSymbolConfiguration(symbolConfig) else {
            let fallback = NSImage(size: NSSize(width: 18, height: 18))
            return fallback
        }

        let brainSize = brainImage.size
        // Extend width slightly for the dot
        let compositeSize = NSSize(width: brainSize.width + 4, height: brainSize.height)

        let composite = NSImage(size: compositeSize, flipped: false) { _ in
            // Draw brain icon in menu bar foreground color
            // Using .labelColor adapts to both light and dark menu bars
            let tintedBrain = brainImage.copy() as! NSImage
            tintedBrain.lockFocus()
            NSColor.labelColor.set()
            NSRect(origin: .zero, size: brainSize).fill(using: .sourceAtop)
            tintedBrain.unlockFocus()

            tintedBrain.draw(
                in: NSRect(origin: .zero, size: brainSize),
                from: .zero,
                operation: .sourceOver,
                fraction: 1.0
            )

            // Draw colored status dot in bottom-right
            let dotSize: CGFloat = 5
            let dotRect = NSRect(
                x: compositeSize.width - dotSize - 0.5,
                y: 1,
                width: dotSize,
                height: dotSize
            )
            let dotColor: NSColor = anyWaiting ? .systemYellow : .systemGreen
            dotColor.setFill()
            NSBezierPath(ovalIn: dotRect).fill()

            return true
        }

        // Non-template so the colored dot is preserved
        composite.isTemplate = false
        return composite
    }
}
