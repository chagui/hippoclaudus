import Foundation
import Testing

@testable import Hippo

@Suite struct FrontmatterParsingTests {

    // MARK: - Helper: write temp file

    private func writeTempFile(_ content: String) -> String {
        let tmp = NSTemporaryDirectory() + UUID().uuidString + ".md"
        try! content.write(toFile: tmp, atomically: true, encoding: .utf8)
        return tmp
    }

    // MARK: - SearchProvider.extractFrontmatter tests

    @Test func validFrontmatterTitleAndTags() {
        let path = writeTempFile("""
        ---
        title: My Great Note
        tags:
          - rust
          - testing
        ---
        # Content here
        """)
        defer { try? FileManager.default.removeItem(atPath: path) }

        let (title, tags) = SearchProvider.extractFrontmatter(filePath: path)
        #expect(title == "My Great Note")
        #expect(tags == ["rust", "testing"])
    }

    @Test func missingTitle() {
        let path = writeTempFile("""
        ---
        tags:
          - rust
        ---
        # No title
        """)
        defer { try? FileManager.default.removeItem(atPath: path) }

        let (title, tags) = SearchProvider.extractFrontmatter(filePath: path)
        #expect(title == nil)
        #expect(tags == ["rust"])
    }

    @Test func inlineTags() {
        let path = writeTempFile("""
        ---
        title: Inline Tags
        tags: [rust, swift, testing]
        ---
        """)
        defer { try? FileManager.default.removeItem(atPath: path) }

        let (title, tags) = SearchProvider.extractFrontmatter(filePath: path)
        #expect(title == "Inline Tags")
        #expect(tags == ["rust", "swift", "testing"])
    }

    @Test func listStyleTags() {
        let path = writeTempFile("""
        ---
        title: List Style
        tags:
          - alpha
          - beta
          - gamma
        ---
        """)
        defer { try? FileManager.default.removeItem(atPath: path) }

        let (title, tags) = SearchProvider.extractFrontmatter(filePath: path)
        #expect(title == "List Style")
        #expect(tags == ["alpha", "beta", "gamma"])
    }

    @Test func noFrontmatter() {
        let path = writeTempFile("# Just a heading\n\nSome content.")
        defer { try? FileManager.default.removeItem(atPath: path) }

        let (title, tags) = SearchProvider.extractFrontmatter(filePath: path)
        #expect(title == nil)
        #expect(tags == [])
    }

    @Test func emptyFile() {
        let path = writeTempFile("")
        defer { try? FileManager.default.removeItem(atPath: path) }

        let (title, tags) = SearchProvider.extractFrontmatter(filePath: path)
        #expect(title == nil)
        #expect(tags == [])
    }

    @Test func malformedYAML() {
        let path = writeTempFile("""
        ---
        title: [unclosed bracket
        tags: {{not yaml}}
        ---
        """)
        defer { try? FileManager.default.removeItem(atPath: path) }

        // Should not crash
        let (_, tags) = SearchProvider.extractFrontmatter(filePath: path)
        let _ = tags
    }

    @Test func nonexistentFile() {
        let (title, tags) = SearchProvider.extractFrontmatter(filePath: "/nonexistent/file.md")
        #expect(title == nil)
        #expect(tags == [])
    }

    // MARK: - VaultTagProvider.extractFrontmatter tests

    @Test func vaultTagProviderValidFrontmatter() {
        let path = writeTempFile("""
        ---
        title: Vault Note
        tags:
          - kubernetes
          - helm
        ---
        """)
        defer { try? FileManager.default.removeItem(atPath: path) }

        let (title, tags) = VaultTagProvider.extractFrontmatter(filePath: path)
        #expect(title == "Vault Note")
        #expect(tags == ["kubernetes", "helm"])
    }

    @Test func vaultTagProviderInlineTags() {
        let path = writeTempFile("""
        ---
        title: Inline
        tags: [a, b, c]
        ---
        """)
        defer { try? FileManager.default.removeItem(atPath: path) }

        let (title, tags) = VaultTagProvider.extractFrontmatter(filePath: path)
        #expect(title == "Inline")
        #expect(tags == ["a", "b", "c"])
    }

    @Test func vaultTagProviderNoFrontmatter() {
        let path = writeTempFile("Plain text file")
        defer { try? FileManager.default.removeItem(atPath: path) }

        let (title, tags) = VaultTagProvider.extractFrontmatter(filePath: path)
        #expect(title == nil)
        #expect(tags == [])
    }

    @Test func quotedTitleAndTags() {
        let path = writeTempFile("""
        ---
        title: "Quoted Title"
        tags:
          - "quoted-tag"
          - 'single-quoted'
        ---
        """)
        defer { try? FileManager.default.removeItem(atPath: path) }

        let (title, tags) = SearchProvider.extractFrontmatter(filePath: path)
        #expect(title == "Quoted Title")
        #expect(tags == ["quoted-tag", "single-quoted"])
    }

    @Test func singleInlineTag() {
        let path = writeTempFile("""
        ---
        title: Single Tag
        tags: solitary
        ---
        """)
        defer { try? FileManager.default.removeItem(atPath: path) }

        let (title, tags) = SearchProvider.extractFrontmatter(filePath: path)
        #expect(title == "Single Tag")
        #expect(tags == ["solitary"])
    }
}
