// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "Hippo",
    platforms: [.macOS(.v14)],
    dependencies: [
        .package(
            url: "https://github.com/swiftlang/swift-testing.git",
            branch: "release/6.0"
        ),
    ],
    targets: [
        .executableTarget(
            name: "Hippo",
            path: "Sources/Hippo",
            linkerSettings: [
                .unsafeFlags([
                    "-Xlinker", "-sectcreate",
                    "-Xlinker", "__TEXT",
                    "-Xlinker", "__info_plist",
                    "-Xlinker", "Info.plist",
                ]),
            ]
        ),
        .testTarget(
            name: "HippoTests",
            dependencies: [
                "Hippo",
                .product(name: "Testing", package: "swift-testing"),
            ],
            path: "Tests/HippoTests"
        ),
    ]
)
