import AppKit

// SPM executables need .regular activation policy to get window server
// access on macOS. We then switch to .accessory to hide the Dock icon.
let app = NSApplication.shared
app.setActivationPolicy(.regular)

// During development, keep .regular so the app shows in the Dock.
// Uncomment for release to hide the Dock icon.
// DispatchQueue.main.async {
//     NSApp.setActivationPolicy(.accessory)
// }

HippoApp.main()
