# Lattice

[![CI](https://github.com/cherishtong/lattice/actions/workflows/ci.yml/badge.svg)](https://github.com/cherishtong/lattice/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-2f6f4e)](LICENSE)

**离线优先、只读、多语言的源码实现阅读平台。**

Lattice 把 CPython、OpenJDK、Rust、TypeScript、SQLite、Nginx、Spring Boot、Vue、React 等真实实现整理成本地 Source Pack，用统一的文件树、符号搜索、定义跳转、引用、调用链、依赖图和版本 Diff 阅读源码。

它不是代码编辑器、IDE、通用仓库分析器，也不是课程平台。

## 范围

Lattice 只做：

- 多语言源码包阅读
- 文件、符号、定义、引用、调用与依赖导航
- 类型继承/实现关系
- 同一实现的跨版本文件 Diff
- 本地注释翻译
- 四套主题与界面缩放
- 离线导入、索引和会话恢复

明确不做：

- 书签、笔记、阅读进度
- Markdown 指南导入或符号锚点
- 用户仓库索引、编辑、调试、构建
- 内建课程或固定学习路径
- 在线翻译、多模型管理、更多语言对
- 自定义主题导入
- 多文件/整包 Diff
- 多窗口、分屏、自动更新、界面国际化

## 已实现功能

- **源码包**：目录/ZIP 导入、校验和、导入取消、卸载、多版本并存。
- **阅读**：tree-sitter 语义高亮、文件树、标签页、文件内查找、虚拟滚动、代码折叠。
- **导航**：全局文件/符号搜索、路径过滤、区分大小写、最近搜索、双击标识符跳转定义。
- **大纲**：C 平铺；Java、JavaScript、TypeScript、Rust、C++ 按类/命名空间/`mod`/`impl`/`trait` 分组，可折叠、键盘导航。
- **引用**：语义引用扫描，排除注释、字符串和条件编译噪声，支持分页、取消和条件分支开关。
- **关系**：直接调用者、被调用函数、多层调用树、文件依赖、反向依赖、模块关系图。
- **类型关系**：Java `extends`/`implements`、JavaScript/TypeScript `extends`/`implements`、C++ 基类、Rust `impl Trait for Type` 与 `trait: bounds`；显示直接基类/接口和跨文件派生/实现者。
- **版本对比**：当前文件与另一版本的逐行 Diff，以及符号级新增/删除/修改摘要。
- **主题与缩放**：无为暗色/纸色、矩阵暗色/浅色；80%–200% 全局缩放。
- **本地翻译**：选中注释，用本地 OPUS-MT 英译中；独立 worker，支持取消、截断提示、错误状态。
- **桌面体验**：自绘标题栏、窗口几何恢复、Material 文件图标、快捷键帮助。

## 源码包

| 实现 | 版本 | 状态 |
| --- | --- | --- |
| CPython | 3.13.2 | 已接入 |
| OpenJDK | 27 (`java.base`) | 已接入 |
| Rust | 1.99.1 | 已接入 |
| TypeScript | 7.0.2 | 已接入 |
| SQLite | 3.54.1 / 3.50.4 | 已接入，多版本对比 |
| Nginx | 1.31.8 | 已接入，`src/` |
| Spring Boot | 4.1.2 | 已接入，`core/`、`module/` 的 `src/main/java` |
| Vue | 3.5.44 | 已接入，`packages/*/src` |
| React | 19.3.1 | 已接入，核心包的 `src/` |

源码包以 ZIP 归档放在仓库的 `source-packs/`，解压后位于 `.content/`。归档不包含符号索引和文档缓存，首次打开时自动重建。

克隆仓库后一键安装：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/install-source-packs.ps1
```

只安装部分包：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/install-source-packs.ps1 -Pack cpython-3.13.2,rust-1.99.1
```

已存在同名目录时默认跳过；需要覆盖时加 `-Force`。

## 安装

从 GitHub Releases 下载：

- `Lattice-<version>-windows-x64.zip`：便携版，解压即用
- `Lattice-<version>-Setup.exe`：当前用户安装版，不需要管理员权限
- 同名 `.sha256` 文件用于校验下载

详细步骤、静默安装、升级和卸载见 [`docs/install.md`](docs/install.md)。

安装包不包含源码包和翻译模型；Source Pack 按上面的脚本安装，模型单独获取。

## 构建

要求：

- Rust 1.88+
- Windows x64 + WebView2
- 首次构建需要网络下载 ONNX Runtime 二进制；之后可 `--offline`

```powershell
cargo build --release -p lattice -p lattice-translator
cargo run -p lattice
```

测试：

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## 重新获取源码包

仓库内的 `source-packs/` 已提供固定版本归档，日常使用不需要重新下载。以下脚本用于重新获取、更新或复现归档内容：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/fetch-cpython-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-openjdk-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-rust-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-typescript-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-sqlite-pack.ps1 -Version 3.54.1
powershell -ExecutionPolicy Bypass -File scripts/fetch-sqlite-pack.ps1 -Version 3.50.4
powershell -ExecutionPolicy Bypass -File scripts/fetch-nginx-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-spring-boot-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-vue-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-react-pack.ps1
```

重新生成 `source-packs/` 归档：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/package-source-packs.ps1
```

预构建索引和文档缓存：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/prepare-cpython-index.ps1 -Pack .content/rust-1.99.1
```

## 本地翻译

模型单独获取，不随安装包发布：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/fetch-opus-mt-model.ps1
```

默认安装到 `.content/models/opus-mt-en-zh`。完整说明见 [`docs/translation.md`](docs/translation.md)。

## 打包

```powershell
# 便携 ZIP + SHA-256
powershell -ExecutionPolicy Bypass -File scripts/package-release.ps1

# 追加 Inno Setup 安装器
powershell -ExecutionPolicy Bypass -File scripts/package-release.ps1 -Installer
```

输出在 `dist/`：版本化 ZIP、`Lattice-<version>-Setup.exe` 和对应 `.sha256`。安装器是 per-user 安装，默认目录 `%LOCALAPPDATA%\Programs\Lattice`。

可选代码签名：设置 `LATTICE_SIGN_CERT_SHA1` 并提供 `signtool.exe`，打包脚本会签名两个 EXE 和安装器。

## 贡献

- 提交前运行 `cargo fmt`、`clippy`、`cargo test`。
- 保持离线阅读的产品边界；不要加入笔记、指南、IDE 或模型市场。
- Source Pack 必须固定上游版本并提供 manifest 和 checksum。
- 详见 [`CONTRIBUTING.md`](CONTRIBUTING.md) 和 [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)。
- 安全漏洞请按 [`SECURITY.md`](SECURITY.md) 私下报告。

## 许可证

应用代码使用 MIT License，见 [`LICENSE`](LICENSE)。

Source Pack、翻译模型、图标和运行时组件使用各自的许可证，汇总见
[`THIRD_PARTY_LICENSES.md`](THIRD_PARTY_LICENSES.md)。Source Pack 归档内保留上游许可证文件。

## 文档

- [产品定义](docs/product.md)
- [架构](docs/architecture.md)
- [路线图](docs/roadmap.md)
- [安装与升级](docs/install.md)
- [Source Pack](docs/source-pack.md)
- [本地翻译](docs/translation.md)
- [主题](docs/themes.md)
- [体积基线](docs/size-budget.md)
- [变更日志](CHANGELOG.md)
- [Native / Blitz（实验）](docs/native-renderer.md)

## 目录

```text
apps/lattice/             Dioxus Desktop 应用
crates/lattice-content/   Source Pack 与只读文件访问
crates/lattice-parser/    tree-sitter 解析、高亮、大纲、引用、类型关系
crates/lattice-query/     文件/符号索引、搜索、定义与关系查询
crates/lattice-translator/ OPUS-MT ONNX 翻译 worker
scripts/                  获取源码包、模型、索引、打包
```
