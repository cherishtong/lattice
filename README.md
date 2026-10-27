# Lattice

**离线优先、多语言、只读的源码阅读与分析平台。**

Lattice 将 CPython、OpenJDK、TypeScript、Rust、GCC/Clang 等语言实现与标准库整理为本地 Source Pack，用统一的文件导航、符号查询和源码阅读帮助开发者理解语言内部机制。

它不是用户仓库分析器，也不是编辑器或 IDE。产品与架构定义见：

- [`docs/product.md`](docs/product.md)
- [`docs/architecture.md`](docs/architecture.md)
- [`docs/roadmap.md`](docs/roadmap.md)
- [`docs/native-renderer.md`](docs/native-renderer.md)
- [`docs/size-budget.md`](docs/size-budget.md)
- [`docs/source-pack.md`](docs/source-pack.md)
- [`docs/themes.md`](docs/themes.md)
- [`docs/translation.md`](docs/translation.md)
- [文件图标库](apps/lattice/assets/file-icons/README.md)

## 当前状态：多语言源码阅读

已经接入五个真实源码包，并提供 GCC 与 Clang 的获取脚本：

- **CPython 3.13.2**：C 源码，支持高亮、大纲、索引、定义与引用。
- **OpenJDK 27**：`java.base` Java 源码，支持高亮、类/方法大纲、索引与跨文件定义。
- **Rust 1.99.1**：`compiler/`、`library/` 与工具源码，支持高亮、Trait/模块/函数大纲、索引与定义。
- **TypeScript 7.0.2**：编译器、服务与标准库声明源码，支持高亮、类/接口/类型/方法大纲、索引与定义。
- **SQLite 3.54.1 / 3.50.4**：官方完整源码树，C 高亮、函数/结构体/宏大纲、索引与定义；两个版本可用于版本对比。
- **GCC 15 / Clang 20**：提供 `fetch-gcc-pack.ps1`、`fetch-clang-pack.ps1`，C++ 使用 tree-sitter-cpp 高亮、类/命名空间/方法大纲。

源码包在“本地源码书库”中按名称与版本直接切换；书库、侧栏、源码头部、文件信息和状态栏都会显示版本号。

- 左侧源码文件树，目录可展开、文件可筛选和打开。
- 中间阅读真实源码，支持 C、Java、Rust 与 TypeScript 的 tree-sitter 高亮。
- 多标签页、文件内搜索和前进/后退导航历史。
- 代码视图按可见行虚拟化渲染，只保留视口附近的 DOM 行；函数、类、结构体、模块等可折叠，跳转到被折叠位置时自动展开；折叠状态跨重启保存，支持 `Alt+[`、`Alt+Shift+[`、`Alt+]`。
- 顶部全局搜索：后台建立整个源码包的符号索引，搜索文件与符号；`↑/↓` 选择、`Enter` 打开并定位，可筛选“全部 / 符号 / 文件”，支持区分大小写、路径过滤与最近搜索。
- 双击代码中的类型、函数、宏或标识符，可跨文件跳到定义；定义查找优先使用当前文件的 import/use/include 路径提示，再按目录与符号种类排序。
- 左侧文件树与标签页使用内嵌的完整 Material Icon Theme 图标集（1,251 个 SVG，按文件名、扩展名和目录名匹配，支持浅色变体）。
- 从大纲选中符号后，可查看跨文件引用；引用经 tree-sitter 校验，自动排除注释与字符串，条件编译分支可一键隐藏，结果支持分页加载与取消扫描。
- “调用链”面板展示当前符号的直接调用者、符号内部调用的函数、文件级导入/包含关系，以及哪些文件依赖当前文件；调用树可逐层展开，模块依赖可切换为关系图。
- 同一实现的多个版本可同时安装在书库中；在非当前版本上点击“对比”，可查看当前文件与目标版本的逐行差异和符号级新增/删除/修改摘要。
- 选中源码注释后，右下角出现“翻译选中”；点击后可用 OPUS-MT 在本地离线翻译成中文，支持取消、截断提示和错误面板，模型与推理均在本机运行。
- 符号大纲按容器嵌套：Java 类、TypeScript 类与命名空间、Rust `mod` / `impl` / `trait`、C++ 类与命名空间下的方法与关联函数会缩进展示；容器可折叠，支持方向键选择、Enter 跳转，并跟随当前代码位置。
- 顶部“本地源码书库”列出已安装源码包，可直接切换、导入或卸载；支持目录与 ZIP 导入，显示复制与索引进度，可随时取消，取消复制会清理半成品目录。
- 重启应用后恢复上次源码包、标签页、活动文件、滚动位置、主题、界面缩放与窗口位置/大小。
- 无系统标题栏：窗口使用自绘标题栏，提供拖动区、最小化、最大化/还原和关闭。
- 键盘快捷键：`Ctrl+K` 全局搜索、`Ctrl+F` 文件内查找、`Ctrl+Tab` 切换标签、`Ctrl+W` 关闭标签、`Alt+←/→` 前进后退、`Ctrl+=`/`Ctrl+-`/`Ctrl+0` 缩放界面、`Alt+[` 折叠当前、`Alt+Shift+[` 折叠全部、`Alt+]` 展开全部、`?` 或 `Ctrl+/` 打开快捷键帮助、`Esc` 关闭面板。
- 在文件树、代码区或符号大纲上按住 `Ctrl` 滚动鼠标滚轮，可放大或缩小整个阅读界面；缩放比例会随会话保存，并显示在状态栏。
- 右侧展示真实文件内符号大纲：Java/TypeScript/Rust/C++ 按容器嵌套方法，C 保持平铺；可筛选、折叠容器、键盘导航并点击跳转到符号所在行。
- 双击符号时优先选择当前行所属符号，并按同目录、同包与符号种类排序跨文件定义。
- “文件信息”展示源码包、路径、语言、行数、符号数与离线状态。
- 文件扫描和解析在后台任务中完成，不阻塞 UI。
- 产品不内建学习路径；后续可导入外部 Markdown 指南作为可选旁注层。

首次运行前准备离线源码包：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/fetch-cpython-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-openjdk-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-rust-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-typescript-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-sqlite-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-gcc-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-clang-pack.ps1
```

本地注释翻译模型（可选，约 117 MiB）：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/fetch-opus-mt-model.ps1
```

CPython 解压到 `.content/cpython-3.13.2`，约 102 MiB；OpenJDK 27 的 `java.base` 核心源码解压到 `.content/openjdk-jdk-27`，约 45 MiB；Rust 1.99.1 解压到 `.content/rust-1.99.1`，约 160 MiB；TypeScript 7.0.2 的编译器源码解压到 `.content/typescript-7.0.2`，约 34 MiB。所有源码包都不进入 Git。

全局索引会自动缓存到源码包的 `.lattice/symbol-index-v2.postcard`。首次运行构建一次，之后 release 下可直接加载，不再重复解析全部源码文件。文档语法高亮结果也会缓存到 `.lattice/documents/`，打开文件时直接恢复行高亮和符号大纲。也可以提前构建：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/prepare-cpython-index.ps1 -Pack .content/rust-1.99.1
```

脚本会为指定源码包构建符号索引和文档缓存。源码包还可以带可选 `checksum` 字段；导入时会校验，校验和可用以下脚本写入：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/stamp-pack-checksums.ps1 -Offline
```

打包与安装器：

```powershell
# 生成 dist/Lattice-<版本>-windows-x64.zip
powershell -ExecutionPolicy Bypass -File scripts/package-release.ps1

# 额外生成 Inno Setup 安装器（需要安装 Inno Setup）
powershell -ExecutionPolicy Bypass -File scripts/package-release.ps1 -Installer
```

技术栈：Rust + Dioxus 0.7 Desktop + tree-sitter C/C++/Java/Rust/TypeScript；界面资源全部本地打包，无 CDN。原型通过 `include_str!` 内嵌 CSS 与图标，因此直接 `cargo run` 也不需要额外的资源处理步骤。

## 运行

Windows 需要 WebView2（现代 Windows 通常已自带）。

```powershell
# 默认：WebView2 后端，稳定路径
cargo run -p lattice

# 实验：Dioxus Native / Blitz GPU 渲染
powershell -ExecutionPolicy Bypass -File scripts/run-native.ps1
```

主题在右上角切换，提供两种风格、各含暗色与纸白：**无为 · 暗 / 无为 · 纸**（墨色、宣纸、朱砂、衬线标题）和 **矩阵 · 暗 / 矩阵 · 白**（终端绿、等宽字形）。主题随阅读会话保存。所有颜色、阴影、语法色与标题字体都由 `themes.css` 的 CSS 变量提供，`app.css` 不写任何字面颜色，后续可直接增加或覆盖自定义主题令牌。会话文件位于 `%APPDATA%\Lattice\session.json`（可用 `LATTICE_CONFIG_DIR` 覆盖目录），不写入源码包。

测量两种后端的 release 产物与 ZIP 体积：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/measure-size.ps1
```

当前基线：WebView2 **13.84 MiB EXE / 3.34 MiB ZIP**；Native **26.81 MiB EXE / 9.73 MiB ZIP**。完整离线 WebView2 还需单独预算约 **205 MiB** 的运行时，见 [`docs/size-budget.md`](docs/size-budget.md)。

也可以使用 Dioxus CLI（如果本机已经安装）：

```powershell
cd apps/lattice
dx serve --desktop
```

## 检查

```powershell
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## 当前目录

```text
apps/lattice/
  assets/app.css        页面布局与组件样式（只使用主题令牌）
  assets/themes.css     四套主题、语义色令牌与主题选择器
  src/app.rs            学习工作台
  src/source.rs         真实源码包会话与后台加载
  src/components/       标题栏、源码文件树、源码视图、符号大纲与图标
crates/lattice-content/ Source Pack 发现、文件树、安全只读访问、ZIP 导入与校验和
crates/lattice-parser/  C/C++/Java/Rust/TypeScript 语法解析、高亮与文件内符号提取
scripts/                获取源码包、运行 Native、测量体积
docs/                   产品、架构和路线定义
```
