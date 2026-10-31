# Changelog

## 0.1.0 - 2026-10-31

First public release.

### Reading

- Immutable Source Packs with manifest, version, license, and optional CRC32
- Source Pack library: directory/ZIP import, validation, cancellation, uninstall
- File tree, tabs, history, in-file search, virtual scrolling, code folding
- tree-sitter highlighting and outline for C, C++, Java, JavaScript, Rust,
  TypeScript/TSX
- Global file and symbol search, definition jump, semantic references
- Callers, callees, call tree, file dependencies, reverse dependencies
- Type relations: Java/JavaScript/TypeScript extends/implements, C++ bases,
  Rust impl/trait bounds, cross-file derived types
- Current-file diff across versions with symbol-level summary
- Four built-in themes, 80%–200% zoom, session restore
- Local English → Chinese OPUS-MT translation in a separate worker

### Content

- CPython 3.13.2
- OpenJDK 27 (`java.base`)
- Rust 1.99.1
- TypeScript 7.0.2
- SQLite 3.54.1 and 3.50.4
- Nginx 1.31.8
- Spring Boot 4.1.2
- Vue 3.5.44
- React 19.3.1

### Distribution

- Portable Windows x64 ZIP
- Inno Setup installer
- Compressed Source Pack archives in `source-packs/`
- Installer/uninstaller, SHA-256 checksums, GitHub Actions CI and release flow
- Third-party license inventory
