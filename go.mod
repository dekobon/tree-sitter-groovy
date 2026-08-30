module github.com/dekobon/tree-sitter-groovy

go 1.23

// v0.25.0's git tag was deleted upstream (tree-sitter/go-tree-sitter#50),
// so GitHub's tag list stops at v0.24.0 and `@latest` resolves there.
// The version is real — commit adc13ff, tagged 2025-02-02 — and stays
// fetchable because proxy.golang.org caches module versions immutably
// and go.sum pins its hashes. Builds with the default GOPROXY are fine;
// GOPROXY=direct cannot resolve it. Do not "fix" this by downgrading to
// v0.24.0: that targets an older tree-sitter core than the committed
// ABI-15 parser.
require github.com/tree-sitter/go-tree-sitter v0.25.0

require github.com/mattn/go-pointer v0.0.1 // indirect
