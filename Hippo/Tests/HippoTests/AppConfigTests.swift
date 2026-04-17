@testable import Hippo
import Testing

struct AppConfigTests {
    @Test func defaults() {
        let vault = AppConfig.vaultPath
        let projects = AppConfig.projectsPath
        #expect(!vault.isEmpty)
        #expect(!projects.isEmpty)
        #expect(vault.contains("Obsidian"))
        #expect(projects.contains(".claude/projects"))
    }

    @Test func vaultNameIsNonEmpty() {
        // Either a configured value or the fallback (lastPathComponent of vault_path) yields non-empty.
        #expect(!AppConfig.vaultName.isEmpty)
    }
}
