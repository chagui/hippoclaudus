import Testing

@testable import Hippo

@Suite struct AppConfigTests {
    @Test func defaults() {
        let vault = AppConfig.vaultPath
        let projects = AppConfig.projectsPath
        #expect(!vault.isEmpty)
        #expect(!projects.isEmpty)
        #expect(vault.contains("Obsidian"))
        #expect(projects.contains(".claude/projects"))
    }
}
