import AppKit

// SPM executables need .regular activation policy to get window server
// access on macOS. We then switch to .accessory to hide the Dock icon.
let app = NSApplication.shared
app.setActivationPolicy(.regular)

DispatchQueue.main.async {
    NSApp.setActivationPolicy(.accessory)
}

ClaudePulseApp.main()
