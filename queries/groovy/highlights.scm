; tree-sitter-groovy highlight queries.
;
; Capture names follow nvim-treesitter conventions:
; https://neovim.io/doc/user/treesitter.html#treesitter-highlight-groups
;
; This file will be populated as grammar.js stabilises. The captures
; below are placeholders that match the minimal stub grammar and will
; expand to cover all of SPECIFICATION.md §9.1.

(line_comment) @comment
(block_comment) @comment
(groovydoc_comment) @comment.documentation

(identifier) @variable
