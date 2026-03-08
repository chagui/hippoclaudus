// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "ClaudePulse",
    platforms: [.macOS(.v14)],
    dependencies: [
        .package(
            url: "https://github.com/swiftlang/swift-testing.git",
            branch: "release/6.0"
        ),
    ],
    targets: [
        .executableTarget(
            name: "ClaudePulse",
            path: "Sources/ClaudePulse"
        ),
        .testTarget(
            name: "ClaudePulseTests",
            dependencies: [
                "ClaudePulse",
                .product(name: "Testing", package: "swift-testing"),
            ],
            path: "Tests/ClaudePulseTests"
        ),
    ]
)
