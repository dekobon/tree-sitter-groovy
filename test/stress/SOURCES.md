# Stress-corpus sources

Each file in this directory is a synthetic Groovy snippet authored
for this project (dual-licensed under Apache-2.0 OR MIT, same as the
parent grammar) or adapted from public-domain examples. The stress test
(`bindings/rust/tests/parse_stress.rs`) walks this directory and
asserts the parser produces zero `ERROR` and zero `MISSING` nodes
for every file.

| File | Coverage focus |
|------|---------------|
| `arithmetic_and_ranges.groovy` | numeric literals, binary expressions, range_expression, parenthesisation |
| `class_with_methods.groovy` | class declaration, multiple methods, formal parameters with types and defaults |
| `closures_and_lists.groovy` | closures, lists, maps, method invocations with closure arguments |
| `control_flow.groovy` | if/else, while, for-in, switch (classic + arrow), try-catch-finally with multi-catch |
| `imports_and_package.groovy` | package, plain / static / wildcard / aliased imports |
| `operators_grab_bag.groovy` | dot-family access, regex match / find, spaceship, identity, Elvis, ternary, range |
| `generics.groovy` | `generic_type`, `type_arguments`, class / interface / trait `type_parameters`, method `method_type_parameters`, wildcards (`?`, `? extends`, `? super`), nested and qualified generic bases |
| `jenkins_pipeline.groovy` | realistic Jenkinsfile shape — nested `pipeline { stages { stage(...) { steps { ... } } } }` closures, GString interpolation, named-argument command chains |
| `gradle_buildscript.groovy` | Gradle DSL — `plugins { id '...' }`, `dependencies { implementation '...' }`, `tasks.named('test') { ... }`, configuration closures |

Adding new stress files is encouraged — keep them syntactically
valid Groovy that exercises constructs from `SPECIFICATION.md` §3
and §4. Each new file must keep the integration test green
(zero ERROR / zero MISSING).
