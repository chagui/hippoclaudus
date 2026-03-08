@preconcurrency import Foundation

/// Reads shared config from ~/Library/Application Support/com.chagui.hippoclaudus/config.json
/// Falls back to hardcoded defaults if the file doesn't exist or can't be parsed.
enum AppConfig {
    private struct ConfigFile: Decodable {
        let vault_path: String?
        let claude_projects_path: String?
    }

    private static let configPath: String = {
        let home = NSHomeDirectory()
        return "\(home)/Library/Application Support/com.chagui.hippoclaudus/config.json"
    }()

    private static let defaultVaultPath: String = NSString("~/Documents/Obsidian/Vaults/Claude").expandingTildeInPath

    private static let defaultProjectsPath: String = NSString("~/.claude/projects").expandingTildeInPath

    private static let cached: ConfigFile? = {
        guard let data = FileManager.default.contents(atPath: configPath),
              let config = try? JSONDecoder().decode(ConfigFile.self, from: data)
        else { return nil }
        return config
    }()

    static var vaultPath: String {
        if let path = cached?.vault_path {
            return NSString(string: path).expandingTildeInPath
        }
        return defaultVaultPath
    }

    static var projectsPath: String {
        if let path = cached?.claude_projects_path {
            return NSString(string: path).expandingTildeInPath
        }
        return defaultProjectsPath
    }
}
