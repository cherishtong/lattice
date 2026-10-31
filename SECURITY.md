# Security policy

## Supported versions

Lattice 0.1.x on Windows x64 is the current supported line.

## Reporting a vulnerability

Do not open a public issue for a vulnerability. Use GitHub's private
vulnerability reporting for `cherishtong/lattice` (Security → Report a
vulnerability), or contact the maintainer through the repository profile.

Please include:

- affected version and Windows build
- a minimal reproduction
- whether the issue requires a malicious Source Pack, archive, or model
- any crash log or affected file path

## Scope

High-value areas:

- path traversal or symlink escape while opening Source Packs and ZIP imports
- ZIP CRC/path validation and installer extraction
- reading files outside `.content`
- denial of service from very large or malformed source files
- unsafe deserialization of index/document caches
- translation worker protocol and process lifecycle

Lattice does not execute source code from Source Packs. It parses UTF-8 text
with tree-sitter and reads files; it never builds or runs the target project.
