# Third-party licenses

Lattice application code is licensed under the MIT License. The software
bundles or downloads third-party components under their own licenses. This file
summarizes the direct components; transitive Rust crates are pinned in
`Cargo.lock` and carry their licenses in the crate metadata.

## Rust dependencies

| Component | Version | License |
| --- | --- | --- |
| dioxus | 0.7.10 | MIT OR Apache-2.0 |
| wry | 0.53.5 | Apache-2.0 OR MIT |
| tao | 0.34.8 | Apache-2.0 |
| webview2-com | 0.38.2 | MIT |
| ort | 2.0.0-rc.13 | MIT OR Apache-2.0 |
| tree-sitter | 0.25.10 | MIT |
| tree-sitter-c | 0.24.2 | MIT |
| tree-sitter-cpp | 0.23.4 | MIT |
| tree-sitter-java | 0.23.5 | MIT |
| tree-sitter-javascript | 0.25.0 | MIT |
| tree-sitter-rust | 0.24.2 | MIT |
| tree-sitter-typescript | 0.23.2 | MIT |
| serde / serde_json | 1.x | MIT OR Apache-2.0 |
| tokio | 1.x | MIT |
| similar | 3.x | Apache-2.0 |
| crc32fast | 1.x | MIT OR Apache-2.0 |
| flate2 | 1.x | MIT OR Apache-2.0 |
| postcard | 1.x | MIT OR Apache-2.0 |

Full license texts are available from each crate's repository and in the
packages resolved by `Cargo.lock`.

## UI assets and runtime

| Component | License | Notes |
| --- | --- | --- |
| Material Icon Theme 5.39.0 | MIT | File and folder SVG icons embedded by `scripts/vendor-file-icons.mjs` |
| WebView2 Runtime | Microsoft proprietary | Required on Windows; not redistributed in the portable ZIP |
| System fonts | System licenses | The app uses installed UI and code fonts; no font files are bundled |

## Source Packs

Source Pack archives are distributed under the upstream licenses. Each archive
keeps its upstream license file and `.lattice/pack.json` license field.

| Pack | License |
| --- | --- |
| CPython 3.13.2 | PSF-2.0 |
| OpenJDK 27 | GPL-2.0-CE |
| Rust 1.99.1 | MIT OR Apache-2.0 |
| TypeScript 7.0.2 | Apache-2.0 |
| SQLite 3.54.1 / 3.50.4 | Public Domain |
| Nginx 1.31.8 | BSD-2-Clause |
| Spring Boot 4.1.2 | Apache-2.0 |
| Vue 3.5.44 | MIT |
| React 19.3.1 | MIT |

## Translation

| Component | License | Notes |
| --- | --- | --- |
| Helsinki-NLP/opus-mt-en-zh | CC-BY-4.0 | Model is downloaded separately; not included in the repository or installer |
| ONNX Runtime | MIT | Linked into `lattice-translator.exe` |

## Redistribution

When redistributing the portable ZIP or installer, keep this file and the
upstream license files. Source Pack archives are separate downloads and already
contain their upstream licenses.
