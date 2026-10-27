# Lattice 架构（Dioxus 重启版）

## 1. 架构目标

架构服务于源码学习，而不是通用仓库托管：

- Dioxus 负责高表现力、可维护的桌面 UI。
- Rust 领域层负责源码包、查询、学习内容和笔记，不依赖 Dioxus。
- 不可变源码包让离线阅读与缓存语义简单、可验证。
- UI 不直接执行源码扫描、解析、SQL 或大批量搜索。

当前仓库已实现 Dioxus 应用、`lattice-content` 与 `lattice-parser` 的第一条垂直切片；其余 crate 是边界约定，在对应能力开始时再创建。

## 2. 领域模型

### SourcePack

一个不可变、可离线安装的学习内容单元：

```text
SourcePackManifest
├── pack_id                 cpython / openjdk / typescript / rust ...
├── implementation          具体实现，不能只用语言名代替
├── language(s)             实现语言与被实现语言分开记录
├── upstream_version        例如 3.13.2
├── upstream_revision       固定 commit / release 标识
├── source_content_hash     源码快照内容身份
├── source_layout           文件与压缩内容的位置
├── parser_manifest         grammar 与抽取规则版本
├── index_format_version    查询投影格式
└── assets                  图标、字体或其他离线资源
```

源码包一经发布不原地修改。升级源码会产生新修订，避免“同一版本内容悄悄变化”。

### Source Pack 之外的本地数据

- `Note / Bookmark`：学习者本地数据，与原始源码分离。
- `SourceAnchor`：`pack revision + path + symbol identity + fallback range`，用于笔记或外部指南定位。
- `Guide`（可选，后续）：用户导入的 Markdown 文档；不属于 Source Pack，不参与源码完整性校验。
- 内置学习路径不是产品核心模型。外部指南只是旁注层，可以没有，也可以由用户自行提供。

行号只作为显示与降级定位，不能成为笔记或指南锚点的唯一身份。

## 3. 分层

```text
Dioxus Desktop Shell
  页面、组件、快捷键、虚拟列表、主题与无障碍
                  │ Command / ViewModel / Event
                  ▼
Application Services
  书库、阅读会话、搜索、笔记、包安装
                  │
        ┌─────────┴──────────┐
        ▼                    ▼
Notes & Guides          Query & Navigation
笔记/书签/外部指南        符号/引用/调用/大纲/搜索
        │                    │
        └─────────┬──────────┘
                  ▼
Source Pack + Local Storage
不可变源码内容 / 索引与笔记 / 包清单
```

### 依赖方向

- UI 依赖应用服务接口和只读 ViewModel。
- 应用服务依赖领域模型与存储接口。
- 领域层不依赖 Dioxus、WebView、窗口或 CSS。
- 源码解析与索引实现可以被替换，但查询结果语义保持稳定。

## 4. 目标 Cargo 布局

只有在对应垂直切片开始时才创建 crate：

```text
apps/
  lattice/                 Dioxus 桌面应用（当前唯一实现）
crates/
  lattice-domain/          SourcePack、Symbol、Note、SourceAnchor
  lattice-content/         包发现、读取、安装与缓存路径
  lattice-query/           统一符号查询、导航与搜索投影
  lattice-guides/          可选外部 Markdown 指南导入与锚点解析
  lattice-storage/         SQLite 迁移与本地持久化
  lattice-parser/          tree-sitter 与语言适配器
```

当前实现将这些职责限制在 `apps/lattice/src` 的展示组件、`crates/lattice-content` 的只读包访问、`crates/lattice-parser` 的 C/Java/Rust/TypeScript 解析与大纲，以及 `crates/lattice-query` 的内存文件/符号索引中。业务逻辑不进入组件。

## 5. Dioxus 使用约束

Dioxus 的价值是 HTML/CSS 的表达力、组件化和跨平台潜力，不是把业务状态搬进 WebView。

- 主题使用语义色变量，组件不写死主题色；当前提供无为与矩阵两种风格，各含暗色与纸白。
- 渲染器差异（WebView / Blitz）通过根节点的 `data-renderer` 隔离，不进入业务状态。
- 组件只接收只读 props 或明确的事件回调。
- 页面状态与源码包数据分离；组件不直接读取文件或 SQLite。
- CSS、SVG 图标、字体全部随应用打包，不加载在线资源。
- 代码视图必须做可见行窗口化，不能把数万行一次性塞进 DOM。
- 大型架构图单独评估 Canvas/SVG；节点布局在后台完成。
- Windows 首发验证 WebView2；离线安装包需明确 WebView2 分发策略。

## 6. 运行时与一致性

### 阅读

1. UI 请求打开 `pack revision + source anchor`。
2. 后台内容服务读取不可变源码并生成当前文档投影。
3. UI 接收完整 `DocumentView`，再替换当前视图。
4. 旧请求带 `request_id` / `document_id`，不得覆盖新页面。

### 搜索

- 每个查询结果标记来源 `pack_revision` 与 `index_revision`。
- 搜索可以跨包，但结果必须显示实现与版本，避免混淆同名符号。
- 索引缺失时允许后台构建或明确提示，不阻塞纯源码阅读。

### 包安装

1. 下载或导入到临时目录。
2. 校验内容哈希与清单完整性。
3. 在独立目录完成解包和索引准备。
4. 原子更新书库登记，使新修订对 UI 可见。
5. 失败只清理临时内容，不影响已安装版本。

这里不需要沿用旧项目的“用户文件保存 → 增量代次提交”主链路，因为源码包本身不可变。

## 7. 查询与语言适配

统一 UI 不等于抹平语言差异。每个适配器输出：

```text
DocumentParse
├── symbols / outline
├── imports / includes
├── references (candidates)
├── documentation ranges
└── diagnostics / unsupported constructs
```

关系至少区分 `resolved / ambiguous / unresolved / external / dynamic`。宏、模板、动态分派和运行时求值不能被静默包装成确定关系。

第一阶段应优先保证：

- 文件与符号搜索。
- 文件内大纲与定义跳转。
- 导览锚点与真实源码位置一致。
- 引用和调用图只展示有证据的关系。

## 8. 数据归属

| 数据 | 是否不可变 | 丢失后的处理 |
| --- | --- | --- |
| 原始源码与上游修订 | 是 | 从已保留安装介质恢复或重新获取 |
| 官方文档与策划导览 | 随内容修订 | 安装对应内容修订 |
| 符号/搜索索引 | 可重建投影 | 从同一源码修订重建 |
| 学习进度、笔记、书签 | 用户本地资产 | 需要备份，不能当作缓存清理 |
| UI 布局缓存、临时高亮 | 可丢弃 | 重新计算 |

笔记备份和源码包缓存清理必须是两套机制。

## 9. 当前实现范围

当前已实现：

- CPython 3.13.2、OpenJDK 27、Rust 1.99.1、TypeScript 7.0.2 本地不可变源码包与安全相对路径读取。
- 真实文件树、目录展开、文件筛选与文件打开。
- C、Java、Rust、TypeScript 的 tree-sitter 高亮与文件内符号大纲。
- 全局文件/符号搜索、定义跳转、候选引用、标签页和阅读历史。
- 符号索引与文档渲染缓存持久化。
- 会话恢复、界面缩放、完整 Material 文件图标、四套主题。
- 无系统标题栏的自绘窗口与源码包导入/卸载。

尚未包含：外部指南锚点持久化、完整语义级引用、笔记持久化、安装器与打包流程、架构图与版本对比。
