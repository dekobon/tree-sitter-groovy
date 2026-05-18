; Language injections per SPECIFICATION.md §9.2.
;
; Doc comments inject as a documentation language. Slashy and
; dollar-slashy string literals inject as `regex` so editors
; highlight the pattern body.

((groovydoc_comment) @injection.content
 (#set! injection.language "javadoc"))

; Slashy regex `/.../` — the leading `/` is what distinguishes it
; from the plain-string flavours at the query layer (single-,
; triple-single-, double-, triple-double-quoted strings all
; share the `string_literal` node kind).
((string_literal) @injection.content
  (#match? @injection.content "^/[^/]")
  (#set! injection.language "regex"))

; Dollar-slashy regex `$/.../$`.
((string_literal) @injection.content
  (#match? @injection.content "^\\$/")
  (#set! injection.language "regex"))
