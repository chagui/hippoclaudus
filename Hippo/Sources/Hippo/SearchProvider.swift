@preconcurrency import Foundation
import SwiftUI

struct SearchResult: Identifiable {
    let id = UUID()
    let filePath: String
    let fileName: String
    let title: String
    let tags: [String]
    let excerpts: [String]

    var fileNameWithoutExtension: String {
        (fileName as NSString).deletingPathExtension
    }
}

@MainActor
final class SearchProvider: ObservableObject {
    @Published var query: String = ""
    @Published var results: [SearchResult] = []
    @Published var isSearching: Bool = false
    @Published var errorMessage: String?

    private nonisolated static let rgPath = "/opt/homebrew/bin/rg"
    private nonisolated static let searchRoot = AppConfig.vaultPath

    private var searchTask: Task<Void, Never>?

    func search() {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            results = []
            errorMessage = nil
            return
        }

        searchTask?.cancel()
        isSearching = true
        errorMessage = nil

        searchTask = Task {
            do {
                let searchResults = try await Self.runRipgrep(query: trimmed)
                if !Task.isCancelled {
                    self.results = searchResults
                    self.errorMessage = nil
                }
            } catch {
                if !Task.isCancelled {
                    self.results = []
                    self.errorMessage = error.localizedDescription
                    NSLog("Search error: %@", error.localizedDescription)
                }
            }
            if !Task.isCancelled {
                self.isSearching = false
            }
        }
    }

    func clearSearch() {
        query = ""
        results = []
        errorMessage = nil
        searchTask?.cancel()
        isSearching = false
    }

    func openInObsidian(result: SearchResult) {
        let encoded = result.fileNameWithoutExtension.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? result.fileNameWithoutExtension
        let vault = AppConfig.vaultName.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? AppConfig.vaultName
        let urlString = "obsidian://open?vault=\(vault)&file=\(encoded)"
        if let url = URL(string: urlString) {
            NSWorkspace.shared.open(url)
        }
    }

    private static func runRipgrep(query: String) async throws -> [SearchResult] {
        try await Task.detached {
            let process = Process()
            process.executableURL = URL(fileURLWithPath: rgPath)
            process.arguments = [
                "--json",
                "--ignore-case",
                "--type", "md",
                "--glob", "!.*",
                "--glob", "!Templates/",
                query,
                searchRoot,
            ]

            let stdout = Pipe()
            process.standardOutput = stdout
            process.standardError = Pipe()

            try process.run()

            let data = stdout.fileHandleForReading.readDataToEndOfFile()
            process.waitUntilExit()

            guard let outputString = String(data: data, encoding: .utf8) else {
                return []
            }

            let lines = outputString.components(separatedBy: "\n").filter { !$0.isEmpty }

            var matchesByFile: [String: [String]] = [:]

            for line in lines {
                guard let lineData = line.data(using: .utf8),
                      let json = try? JSONSerialization.jsonObject(with: lineData) as? [String: Any],
                      let type = json["type"] as? String,
                      type == "match",
                      let matchData = json["data"] as? [String: Any],
                      let pathObj = matchData["path"] as? [String: Any],
                      let filePath = pathObj["text"] as? String,
                      let linesObj = matchData["lines"] as? [String: Any],
                      let lineText = linesObj["text"] as? String
                else {
                    continue
                }

                let trimmedLine = lineText.trimmingCharacters(in: .whitespacesAndNewlines)
                if !trimmedLine.isEmpty {
                    matchesByFile[filePath, default: []].append(trimmedLine)
                }
            }

            var results: [SearchResult] = []

            for (filePath, excerpts) in matchesByFile {
                let fileName = (filePath as NSString).lastPathComponent
                let (title, tags) = extractFrontmatter(filePath: filePath)
                let limitedExcerpts = Array(excerpts.prefix(3))

                results.append(SearchResult(
                    filePath: filePath,
                    fileName: fileName,
                    title: title ?? (fileName as NSString).deletingPathExtension,
                    tags: tags,
                    excerpts: limitedExcerpts,
                ))
            }

            results.sort { $0.title.localizedCaseInsensitiveCompare($1.title) == .orderedAscending }
            return Array(results.prefix(20))
        }.value
    }

    nonisolated static func extractFrontmatter(filePath: String) -> (title: String?, tags: [String]) {
        guard let fileHandle = FileHandle(forReadingAtPath: filePath) else {
            return (nil, [])
        }
        defer { fileHandle.closeFile() }

        let chunk = fileHandle.readData(ofLength: 2048)
        guard let content = String(data: chunk, encoding: .utf8) else {
            return (nil, [])
        }

        let lines = content.components(separatedBy: "\n")
        guard lines.first?.trimmingCharacters(in: .whitespaces) == "---" else {
            return (nil, [])
        }

        var title: String?
        var tags: [String] = []
        var inTags = false

        for line in lines.dropFirst().prefix(15) {
            let trimmed = line.trimmingCharacters(in: .whitespaces)

            if trimmed == "---" {
                break
            }

            if trimmed.hasPrefix("title:") {
                let value = trimmed.dropFirst(6).trimmingCharacters(in: .whitespaces)
                title = value.trimmingCharacters(in: CharacterSet(charactersIn: "\"'"))
                inTags = false
            } else if trimmed.hasPrefix("tags:") {
                let inline = trimmed.dropFirst(5).trimmingCharacters(in: .whitespaces)
                if inline.hasPrefix("[") {
                    let inner = inline.trimmingCharacters(in: CharacterSet(charactersIn: "[]"))
                    tags = inner.components(separatedBy: ",").map {
                        $0.trimmingCharacters(in: .whitespaces).trimmingCharacters(in: CharacterSet(charactersIn: "\"'"))
                    }.filter { !$0.isEmpty }
                    inTags = false
                } else if inline.isEmpty {
                    inTags = true
                } else {
                    tags = [inline.trimmingCharacters(in: CharacterSet(charactersIn: "\"'"))]
                    inTags = false
                }
            } else if inTags, trimmed.hasPrefix("- ") {
                let tag = String(trimmed.dropFirst(2)).trimmingCharacters(in: .whitespaces).trimmingCharacters(in: CharacterSet(charactersIn: "\"'"))
                if !tag.isEmpty {
                    tags.append(tag)
                }
            } else if !trimmed.isEmpty, !trimmed.hasPrefix("-") {
                inTags = false
            }
        }

        return (title, tags)
    }
}
