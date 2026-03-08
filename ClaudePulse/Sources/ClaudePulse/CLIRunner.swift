@preconcurrency import Foundation

enum CLIRunner {
    static let primaryPath = NSString("~/.local/bin/claude-pulse").expandingTildeInPath

    /// Default timeout for CLI operations (30 seconds).
    static let defaultTimeout: UInt64 = 30_000_000_000 // nanoseconds

    static func resolvedBinaryPath() -> String? {
        let fm = FileManager.default
        if fm.isExecutableFile(atPath: primaryPath) {
            return primaryPath
        }
        return nil
    }

    static func run(arguments: [String], timeout: UInt64 = defaultTimeout) async throws -> Data {
        guard let binaryPath = resolvedBinaryPath() else {
            throw CLIRunnerError.binaryNotFound
        }

        return try await withThrowingTaskGroup(of: Data.self) { group in
            group.addTask {
                try await withTaskCancellationHandler {
                    try await Task.detached {
                        let process = Process()
                        process.executableURL = URL(fileURLWithPath: binaryPath)
                        process.arguments = arguments

                        let stdout = Pipe()
                        process.standardOutput = stdout
                        process.standardError = Pipe()

                        try process.run()
                        process.waitUntilExit()

                        let data = stdout.fileHandleForReading.readDataToEndOfFile()

                        guard process.terminationStatus == 0 else {
                            throw CLIRunnerError.nonZeroExit(Int(process.terminationStatus))
                        }

                        return data
                    }.value
                } onCancel: {
                    // Process will be cleaned up when Task.detached is cancelled
                }
            }

            group.addTask {
                try await Task.sleep(nanoseconds: timeout)
                throw CLIRunnerError.timeout
            }

            let result = try await group.next()!
            group.cancelAll()
            return result
        }
    }

    /// Runs the CLI without a timeout. The process is terminated if the Task is cancelled.
    static func runCancellable(arguments: [String]) async throws -> Data {
        guard let binaryPath = resolvedBinaryPath() else {
            throw CLIRunnerError.binaryNotFound
        }

        let process = Process()
        process.executableURL = URL(fileURLWithPath: binaryPath)
        process.arguments = arguments

        let stdout = Pipe()
        process.standardOutput = stdout
        process.standardError = Pipe()

        return try await withTaskCancellationHandler {
            try await Task.detached {
                try process.run()
                process.waitUntilExit()

                let data = stdout.fileHandleForReading.readDataToEndOfFile()

                guard process.terminationStatus == 0 else {
                    throw CLIRunnerError.nonZeroExit(Int(process.terminationStatus))
                }

                return data
            }.value
        } onCancel: {
            process.terminate()
        }
    }

    static func runStreaming(arguments: [String], onLine: @escaping @Sendable (String) -> Void) async throws {
        guard let binaryPath = resolvedBinaryPath() else {
            throw CLIRunnerError.binaryNotFound
        }

        try await Task.detached {
            let process = Process()
            process.executableURL = URL(fileURLWithPath: binaryPath)
            process.arguments = arguments

            let stdout = Pipe()
            process.standardOutput = stdout
            process.standardError = Pipe()

            try process.run()

            // Use blocking reads with proper EOF detection instead of availableData
            let handle = stdout.fileHandleForReading
            var buffer = Data()

            while true {
                let chunk = handle.readData(ofLength: 4096)
                if chunk.isEmpty {
                    break // EOF
                }
                buffer.append(chunk)

                while let newlineRange = buffer.range(of: Data([0x0A])) {
                    let lineData = buffer.subdata(in: buffer.startIndex..<newlineRange.lowerBound)
                    buffer.removeSubrange(buffer.startIndex...newlineRange.lowerBound)
                    if let line = String(data: lineData, encoding: .utf8) {
                        onLine(line)
                    }
                }
            }

            if !buffer.isEmpty, let remaining = String(data: buffer, encoding: .utf8) {
                onLine(remaining)
            }

            process.waitUntilExit()

            guard process.terminationStatus == 0 else {
                throw CLIRunnerError.nonZeroExit(Int(process.terminationStatus))
            }
        }.value
    }
}

enum CLIRunnerError: Error, LocalizedError {
    case binaryNotFound
    case nonZeroExit(Int)
    case timeout

    var errorDescription: String? {
        switch self {
        case .binaryNotFound:
            return "claude-pulse binary not found at \(CLIRunner.primaryPath). Run install.sh first."
        case .nonZeroExit(let code):
            return "claude-pulse exited with code \(code)"
        case .timeout:
            return "claude-pulse timed out"
        }
    }
}
