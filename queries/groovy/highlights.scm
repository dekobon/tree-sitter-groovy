; tree-sitter-groovy highlight queries.
;
; Capture names follow nvim-treesitter conventions:
; https://neovim.io/doc/user/treesitter.html#treesitter-highlight-groups
;
; Precedence rule: tree-sitter highlight queries are last-match-wins
; per node. Layers below are ordered from broadest to most specific:
;   1. comments / number / string / boolean / null
;   2. anonymous-token operators (also captures . , ?., etc.)
;   3. generic identifier → @variable  (the catch-all)
;   4. keywords (@keyword, @keyword.control, @keyword.operator)
;   5. field / property captures off the dot family
;   6. declared names (class / method / record / formal parameter)
;   7. call-site captures (@function.call) — must come AFTER property
;      so a method-named field beats the property capture
;   8. type uses (@type on type_identifier) — must come AFTER
;      @variable so aliased identifier-as-type wins
;   9. annotation usage (the @attribute capture)

; -- Layer 1: comments and literals --------------------------------

(line_comment) @comment
(block_comment) @comment
(groovydoc_comment) @comment.documentation

(number_literal) @number
(string_literal) @string
(string_fragment) @string
(boolean_literal) @boolean
(null_literal) @constant.builtin

; GString interpolation — `${expr}` and `$identifier` (per §5.7).
(gstring_brace_interpolation
  "${" @punctuation.special
  "}" @punctuation.special)
; The `->` of a lazy interpolation (`"${-> x}"`) is a `closure_parameters`
; leaf of its own, which the operator list below does not reach.
(gstring_brace_interpolation
  (closure_parameters) @operator)
(gstring_dollar_interpolation
  value: (identifier) @variable)

; -- Layer 2: operators (anonymous tokens) -------------------------

[
  "+"
  "-"
  "*"
  "/"
  "%"
  "!"
  "~"
  "<<"
  ">>"
  ">>>"
  "<"
  "<="
  ">"
  ">="
  "=="
  "!="
  "&"
  "^"
  "|"
  "&&"
  "||"
  ".."
  "..<"
  "<.."
  "<..<"
  "==="
  "!=="
  "<=>"
  "=~"
  "==~"
  "?:"
  "?"
  "==>"
  "="
  "+="
  "-="
  "*="
  "/="
  "%="
  "**="
  "<<="
  ">>="
  ">>>="
  "&="
  "^="
  "|="
  "?="
  "**"
  "++"
  "--"
  "."
  "?."
  "??."
  "*."
  ".&"
  ".@"
  "?.@"
  "??.@"
  "*.@"
  "::"
  "*:"
  "->"
  "..."
] @operator

; A slashy string's closing `/` is the same anonymous symbol as the
; division operator, so the list above captures it too; this later
; match gives it back to the string, like the hidden opening `/`.
(string_literal "/" @string)

; -- Layer 3: generic identifier fallback --------------------------

(identifier) @variable

; -- Layer 4: keywords ---------------------------------------------

[
  "new"
  "as"
  "in"
  "instanceof"
] @keyword.operator

; `!instanceof` is a composite token (literal + trailing whitespace)
; per SPECIFICATION.md §3.2.2, so it cannot be referenced as a direct
; anonymous token in a `[ … ]` group. Capture it, and `!in` with it,
; through the operator field instead.
(membership_expression operator: _ @keyword.operator)
(instanceof_expression operator: _ @keyword.operator)

[
  "if"
  "else"
  "while"
  "do"
  "for"
  "return"
  "break"
  "continue"
  "throw"
  "throws"
  "assert"
  "try"
  "catch"
  "finally"
  "switch"
  "case"
  "default"
  "yield"
] @keyword.control

[
  "class"
  "trait"
  "interface"
  "enum"
  "record"
  "def"
  "var"
  "package"
  "import"
  "pipeline"
  "sealed"
  "non-sealed"
  "permits"
  "extends"
  "implements"
] @keyword

[
  "public"
  "private"
  "protected"
  "static"
  "final"
  "abstract"
  "synchronized"
  "native"
  "transient"
  "volatile"
  "strictfp"
] @keyword.modifier

; `synchronized (lock) { … }` is a statement; the word is a modifier
; everywhere else.
(synchronized_statement "synchronized" @keyword)

; `default` is a branch label in a `switch` and a modifier on a method
; or a type declaration (`default void run() { }`). On a method that has
; a `default` field it gives an annotation element its default
; (`String value() default 'x'`) instead; a method never has both.
(method_declaration !default "default" @keyword.modifier)
(method_declaration "default" @keyword default: (_))
[
  (class_declaration "default" @keyword.modifier)
  (interface_declaration "default" @keyword.modifier)
  (trait_declaration "default" @keyword.modifier)
  (enum_declaration "default" @keyword.modifier)
  (record_declaration "default" @keyword.modifier)
]

(shebang) @keyword.directive

; -- Layer 5: dot-family property captures -------------------------

(field_access                    field: [(identifier) (quoted_identifier)] @property)
(safe_navigation_expression     property: [(identifier) (quoted_identifier)] @property)
(safe_chain_dot_expression      property: [(identifier) (quoted_identifier)] @property)
(spread_dot_expression          property: [(identifier) (quoted_identifier)] @property)
(direct_field_access_expression  field: [(identifier) (quoted_identifier)] @property)
(safe_direct_field_access_expression        field: [(identifier) (quoted_identifier)] @property)
(safe_chain_direct_field_access_expression  field: [(identifier) (quoted_identifier)] @property)
(spread_direct_field_access_expression      field: [(identifier) (quoted_identifier)] @property)
(method_pointer_expression      method: [(identifier) (quoted_identifier)] @function)
; `Foo::new` — the `new` literal is captured as @keyword.operator at
; layer 4 instead of @function here.
(method_reference_expression       name: (identifier) @function)

; -- Layer 6: declared names ---------------------------------------

(class_declaration name: (identifier) @type.definition)
(trait_declaration name: (identifier) @type.definition)
(interface_declaration name: (identifier) @type.definition)
(enum_declaration name: (identifier) @type.definition)
(record_declaration name: (identifier) @type.definition)
(annotation_type_declaration name: (identifier) @type.definition)
(enum_constant name: (identifier) @constant)
(method_declaration name: [(identifier) (quoted_identifier)] @function)
(constructor_declaration name: (identifier) @constructor)
(compact_constructor_declaration name: (identifier) @constructor)
(formal_parameter name: (identifier) @variable.parameter)
(closure_parameter name: (identifier) @variable.parameter)
(lambda_expression parameters: (identifier) @variable.parameter)
(record_component name: (identifier) @variable.parameter)
(variable_declarator name: (identifier) @variable)
; The key of a keyed pair names the entry or property it takes from the
; value, not a variable: `def (name: n, age: int a) = person`.
(variable_declarator key: (identifier) @property)
(rest_declarator name: (identifier) @variable)
(field_declaration (variable_declarator name: (identifier) @property))

; Labels (declaration + use sites)
(labeled_statement label: (identifier) @label)
(break_statement label: (identifier) @label)
(continue_statement label: (identifier) @label)

; -- Layer 7: call sites (after layer 5 so method-named fields win) -

; A call through `.`, `?.`, `??.` or `*.` names its method in the
; access's last name, which layer 5 made a @property. A call through
; `.&` keeps @function, as the name is a method pointer; a call through
; `.@` keeps @property, as it calls the value of the field.
(method_invocation
  function: (identifier) @function.call)

(method_invocation
  function: [
    (field_access field: (identifier) @function.call)
    (safe_navigation_expression property: (identifier) @function.call)
    (safe_chain_dot_expression property: (identifier) @function.call)
    (spread_dot_expression property: (identifier) @function.call)
  ])

(command_chain
  receiver: (identifier) @function.call)

(command_chain
  receiver: [
    (field_access field: (identifier) @function.call)
    (safe_navigation_expression property: (identifier) @function.call)
    (safe_chain_dot_expression property: (identifier) @function.call)
    (spread_dot_expression property: (identifier) @function.call)
  ])

; -- Layer 8: types (after @variable so aliased identifier wins) ---

(type_identifier) @type

; Java/Groovy primitive types and `void` highlight as builtin.
((type_identifier) @type.builtin
  (#match? @type.builtin "^(boolean|byte|char|double|float|int|long|short|void)$"))

; -- Layer 9: annotation usage -------------------------------------

(annotation
  "@" @attribute
  name: (qualified_name) @attribute)

; The names and dots inside the qualified name are nodes of their own,
; which layers 2 and 3 captured as @operator and @variable.
(annotation
  name: (qualified_name [(identifier) "."] @attribute))
