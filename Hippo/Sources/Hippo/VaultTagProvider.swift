@preconcurrency import Foundation
import SwiftUI

// MARK: - Models

struct TagDocument: Identifiable {
    let title: String
    let filePath: String
    let fileName: String

    var id: String {
        filePath
    }

    var fileNameWithoutExtension: String {
        (fileName as NSString).deletingPathExtension
    }
}

struct VaultTag: Identifiable {
    let name: String
    let documents: [TagDocument]

    var documentCount: Int {
        documents.count
    }

    var id: String {
        name
    }
}

// MARK: - Provider

@MainActor
final class VaultTagProvider: ObservableObject {
    @Published var tags: [VaultTag] = []

    private nonisolated static let vaultPath = AppConfig.vaultPath

    func refresh() async {
        let results = await Task.detached {
            Self.scanTags()
        }.value
        tags = results
    }

    func openInObsidian(document: TagDocument) {
        let encoded = document.fileNameWithoutExtension
            .addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? document.fileNameWithoutExtension
        let urlString = "obsidian://open?vault=Claude&file=\(encoded)"
        if let url = URL(string: urlString) {
            NSWorkspace.shared.open(url)
        }
    }

    private nonisolated static func scanTags() -> [VaultTag] {
        let fm = FileManager.default

        guard let enumerator = fm.enumerator(atPath: vaultPath) else {
            NSLog("VaultTagProvider: vault path not enumerable: %@", vaultPath)
            return []
        }

        var tagToDocuments: [String: [TagDocument]] = [:]

        while let relativePath = enumerator.nextObject() as? String {
            guard relativePath.hasSuffix(".md"),
                  !relativePath.hasPrefix("."),
                  !relativePath.hasPrefix("Templates/")
            else { continue }

            let fullPath = "\(vaultPath)/\(relativePath)"
            let (title, tags) = extractFrontmatter(filePath: fullPath)
            let fileName = (relativePath as NSString).lastPathComponent
            let displayTitle = title ?? (fileName as NSString).deletingPathExtension

            let doc = TagDocument(title: displayTitle, filePath: fullPath, fileName: fileName)

            for tag in tags {
                tagToDocuments[tag, default: []].append(doc)
            }
        }

        var result = tagToDocuments.map { VaultTag(name: $0.key, documents: $0.value) }
        result.sort { $0.documentCount > $1.documentCount }
        return result
    }

    /// Parses YAML frontmatter to extract title and tags.
    nonisolated static func extractFrontmatter(filePath: String) -> (title: String?, tags: [String]) {
        guard let handle = FileHandle(forReadingAtPath: filePath) else { return (nil, []) }
        defer { handle.closeFile() }

        let chunk = handle.readData(ofLength: 2048)
        guard let content = String(data: chunk, encoding: .utf8) else { return (nil, []) }

        let lines = content.components(separatedBy: "\n")
        guard lines.first?.trimmingCharacters(in: .whitespaces) == "---" else { return (nil, []) }

        var title: String?
        var tags: [String] = []
        var inTags = false

        for line in lines.dropFirst().prefix(15) {
            let trimmed = line.trimmingCharacters(in: .whitespaces)

            if trimmed == "---" { break }

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
                let tag = String(trimmed.dropFirst(2))
                    .trimmingCharacters(in: .whitespaces)
                    .trimmingCharacters(in: CharacterSet(charactersIn: "\"'"))
                if !tag.isEmpty { tags.append(tag) }
            } else if !trimmed.isEmpty, !trimmed.hasPrefix("-") {
                inTags = false
            }
        }

        return (title, tags)
    }
}
