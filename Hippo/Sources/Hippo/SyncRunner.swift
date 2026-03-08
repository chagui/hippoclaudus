@preconcurrency import Foundation
import SwiftUI

@MainActor
final class SyncRunner: ObservableObject {
    @Published var isRunning: Bool = false
    @Published var outputLines: [String] = []
    @Published var errorMessage: String?

    func runSync() async {
        guard !isRunning else { return }
        isRunning = true
        outputLines = []
        errorMessage = nil

        do {
            try await CLIRunner.runStreaming(arguments: ["sync", "--days", "7"]) { [weak self] line in
                Task { @MainActor [weak self] in
                    self?.outputLines.append(line)
                }
            }
        } catch {
            errorMessage = error.localizedDescription
        }

        isRunning = false
    }
}
