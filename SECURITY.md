# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.2.x   | Yes       |

## Reporting a vulnerability

If you find a security vulnerability in `tree-sitter-groovy`, please
report it responsibly through
[GitHub Security Advisories](https://github.com/dekobon/tree-sitter-groovy/security/advisories/new)
rather than opening a public issue.

The most likely class of vulnerability in a tree-sitter parser is a
crash or infinite loop in the external scanner (`src/scanner.c`) when
processing malicious input. The CI pipeline includes a fuzzing job
that runs on every scanner change, but coverage is not exhaustive.

You should receive an acknowledgement within 7 days. A fix will be
released as a patch version as soon as it is ready.
