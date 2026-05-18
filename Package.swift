// swift-tools-version:5.3
import PackageDescription

let package = Package(
    name: "TreeSitterGroovy",
    platforms: [.macOS(.v10_13), .iOS(.v11)],
    products: [
        .library(name: "TreeSitterGroovy", targets: ["TreeSitterGroovy"]),
    ],
    dependencies: [
        .package(url: "https://github.com/ChimeHQ/SwiftTreeSitter", from: "0.8.0"),
    ],
    targets: [
        .target(name: "TreeSitterGroovy",
                path: ".",
                exclude: [
                    "Cargo.toml",
                    "Makefile",
                    "binding.gyp",
                    "bindings/c/tree-sitter-groovy.pc.in",
                    "bindings/go",
                    "bindings/node",
                    "bindings/python",
                    "bindings/rust",
                    "bindings/swift/TreeSitterGroovyTests",
                    "prebuilds",
                    "grammar.js",
                    "package.json",
                    "package-lock.json",
                    "pyproject.toml",
                    "setup.py",
                    "test",
                    "examples",
                    ".editorconfig",
                    ".github",
                    ".gitignore",
                    ".gitattributes",
                    ".gitmodules",
                ],
                sources: [
                    "src/parser.c",
                    "src/scanner.c",
                ],
                resources: [
                    .copy("queries")
                ],
                // Single source of truth for the public header is
                // bindings/c/tree-sitter-groovy.h. The Swift target
                // re-exports it via this publicHeadersPath instead
                // of carrying its own copy.
                publicHeadersPath: "bindings/c",
                cSettings: [.headerSearchPath("src")]),
        .testTarget(name: "TreeSitterGroovyTests",
                    dependencies: [
                        "SwiftTreeSitter",
                        "TreeSitterGroovy",
                    ],
                    path: "bindings/swift/TreeSitterGroovyTests"),
    ],
    cLanguageStandard: .c11
)
