; Language injections per SPECIFICATION.md §9.2.
;
; Doc comments inject as a documentation language. Slashy and
; dollar-slashy string literals inject as `regex` so editors
; highlight the pattern body. A call on `sql` or `Sql` whose first
; argument is a double-quoted string (`sql.execute """..."""`) injects
; that string as `sql`.

((groovydoc_comment) @injection.content
 (#set! injection.language "javadoc"))

; Slashy `/.../` and dollar-slashy `$/.../$` regexes. Every string
; flavour is a `string_literal`; only a slashy one has a `/` child (its
; closer) and only a dollar-slashy one a `$/` child (its opener).
; Per fragment, so no delimiter or interpolation reaches the regex parser.
((string_literal
   (string_fragment) @injection.content
   "/")
 (#set! injection.language "regex"))

((string_literal
   "$/"
   (string_fragment) @injection.content)
 (#set! injection.language "regex"))

; `sql.execute """SELECT ..."""`, `sql.rows("SELECT ...")` and the
; other `groovy.sql.Sql` calls. Matches a call, with or without
; parentheses, of any method on the bare name `sql` or `Sql` whose
; first argument is a `"…"` or `"""…"""` string. Only the first
; argument is SQL: a later string is a bind parameter. A single-quoted
; string has no `string_fragment` to inject into, and a slashy one
; already injects as `regex`. The injection content is each
; literal-fragment range of the string, so `${param}` interpolations
; stay tree-sitter-parsable rather than being swallowed by the SQL
; highlighter.
((method_invocation
   function: (field_access
     object: (identifier) @_recv
     field: (identifier))
   arguments: (argument_list
     .
     (string_literal (string_fragment) @injection.content) @_sql))
 (#match? @_recv "^[Ss]ql$")
 (#match? @_sql "^\"")
 (#set! injection.language "sql"))

((command_chain
   receiver: (field_access
     object: (identifier) @_recv
     field: (identifier))
   .
   argument: (string_literal (string_fragment) @injection.content) @_sql)
 (#match? @_recv "^[Ss]ql$")
 (#match? @_sql "^\"")
 (#set! injection.language "sql"))
