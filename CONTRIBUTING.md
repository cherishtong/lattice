# Contributing

Thanks for improving Lattice. This project is a read-only, offline source
reader; changes should keep reading fast, deterministic, and free of network
access at runtime.

## Before you start

- Read `docs/product.md`, `docs/architecture.md`, and `docs/source-pack.md`.
- Keep the reduced scope: no notes, guides, bookmarks, IDE features, model
  marketplace, extra language pairs, custom theme imports, or multi-file diff.
- For Source Pack changes, update the archive, manifest, checksum, and the
  inventory in `source-packs/README.md`.

## Development

Requirements: Rust 1.88+, Windows x64, WebView2.

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo test --workspace --offline
cargo run -p lattice
```

Install the bundled source packs first:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/install-source-packs.ps1
```

## Pull requests

- Keep changes focused; avoid unrelated formatting churn.
- Add or update tests for parser, query, content, and UI behavior.
- Do not commit `.content/`, `dist/`, `target/`, logs, or local credentials.
- Do not add generated media or large binaries to the repository. Promo videos
  and posters live outside the repository.
- Update documentation when behavior, scope, or pack inventory changes.
- By contributing, you agree that your contribution is licensed under the
  project's MIT License.

## Source Pack rules

- Fixed upstream tag or release; never a moving branch.
- Record `implementation`, `version`, `language`, `source_language`, `license`,
  and `default_entry` in `.lattice/pack.json`.
- Exclude index caches, document caches, and quarantined/corrupt directories
  from `source-packs/*.zip`.
- Run `scripts/package-source-packs.ps1` and update hashes in
  `source-packs/README.md`.

## Reporting issues

Use GitHub Issues. For security problems, follow `SECURITY.md` instead of
opening a public issue.
