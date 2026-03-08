@testable import Hippo
import Testing

struct CLIRunnerErrorTests {
    @Test func binaryNotFoundDescription() {
        let error = CLIRunnerError.binaryNotFound
        let description = error.errorDescription!
        #expect(description.contains("not found"), "Expected 'not found' in: \(description)")
        #expect(description.contains("hpc"), "Expected 'hpc' in: \(description)")
    }

    @Test func nonZeroExitDescription() {
        let error = CLIRunnerError.nonZeroExit(42)
        let description = error.errorDescription!
        #expect(description.contains("42"), "Expected exit code in: \(description)")
        #expect(description.contains("hpc"), "Expected 'hpc' in: \(description)")
    }

    @Test func timeoutDescription() {
        let error = CLIRunnerError.timeout
        let description = error.errorDescription!
        #expect(description.contains("timed out"), "Expected 'timed out' in: \(description)")
    }

    @Test func errorConformsToLocalizedError() {
        let errors: [CLIRunnerError] = [
            .binaryNotFound,
            .nonZeroExit(1),
            .timeout,
        ]

        for error in errors {
            #expect(error.errorDescription != nil, "errorDescription should be non-nil for \(error)")
        }
    }
}
