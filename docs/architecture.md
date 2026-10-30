# Lattice 架构

## 1. 目标

Lattice 是一个离线、只读、多语言的源码实现阅读器。架构目标：

- 源码包不可变、可验证、可离线安装。
- 解析、索引和搜索在后台线程完成，不阻塞 UI。
- 各语言通过解析器适配层输出统一的 Document、Symbol 和关系模型。
- UI 只消费投影数据，不直接扫描目录或解析源码。
- 索引缓存可重建，丢失不影响源码。

## 2. Cargo 布局

```text
apps/lattice/
  Dioxus Desktop + WebView2 应用
  src/app.rs            应用入口、快捷键、会话保存
  src/source.rs         阅读会话、导航、引用、调用链、Diff
  src/config.rs         会话配置
  src/theme.rs          主题
  src/translation.rs    翻译 worker 管理
  src/components/       标题栏、文件树、代码视图、侧栏、翻译面板

crates/lattice-content/
  Source Pack 清单、目录树、只读读取、目录/ZIP 导入、校验和

crates/lattice-parser/
  tree-sitter 解析、高亮、符号大纲、引用、调用点、类型关系

crates/lattice-query/
  文件/符号索引、定义查找、搜索、类型关系查询、索引缓存

crates/lattice-translator/
  OPUS-MT ONNX 推理与 NDJSON worker 协议
```

`lattice-translator` 是独立可执行进程，主程序不静态加载模型，也不把推理放进 UI 线程。

## 3. 数据模型

### SourcePack

```text
SourcePack
├── manifest: id / name / implementation / version / language
├── source_language
├── license
├── default_entry
├── checksum（可选）
└── 不可变源码目录
```

源码包安装后不原地修改；新版本作为新的包版本安装。当前支持目录和 ZIP 导入，读取路径经过相对路径校验。

### Document

```text
Document
├── path
├── language
├── lines: Line[]（带语法 span）
├── symbols: Symbol[]
└── relations: TypeRelation[]
```

`Symbol` 包含名称、种类、起止行和截断后的声明签名。`TypeRelation` 表示从某个类型符号指向另一个类型符号的直接关系：

```text
TypeRelation
├── from / from_line
├── to
└── kind: Extends | Implements | Base
```

类型关系的来源：

- Java：`class ... extends ... implements ...`、`interface ... extends ...`
- JavaScript / TypeScript：`class ... extends ... implements ...`、`interface ... extends ...`
- C++：`class/struct ... : public Base, ...`
- Rust：`impl Trait for Type`、`trait Foo: Bar + Baz`

关系只表达声明中能直接看到的目标，不做跨语言类型推导。未在索引中找到的目标在 UI 中标记为“未解析”。

## 4. 索引

`lattice-query::SourceIndex` 保存：

```text
SourceIndex
├── files: IndexedFile[]
├── symbols: IndexedSymbol[]
└── relations: IndexedRelation[]
```

构建流程：

1. 遍历源码包文件。
2. 按文件路径与内容判定语言（C 头文件含 C++ 结构时按 C++ 处理）。
3. 对每个源码文件调用 parser 大纲，从大纲符号生成类型关系。
4. 去重、排序，写入 `.lattice/symbol-index-v3.postcard`。
5. 下次启动优先加载缓存；版本或指纹不匹配时重建。

关系查询 API：

- `relations_for(name, path, line, language)`
- 返回 `bases`：当前类型的直接基类/接口/Trait
- 返回 `derived`：跨文件派生类型/实现者
- 基类目标通过定义索引解析到路径和行号

索引格式版本升级时，旧缓存自动失效。

## 5. UI 数据流

```text
UI 事件
  → SourceSession / TranslationSession
  → 后台任务（读取、解析、索引、引用扫描）
  → Signal / ViewModel
  → 组件渲染
```

约束：

- 文件 I/O、tree-sitter 解析、索引构建不得在 UI 线程执行。
- 代码视图只渲染可见行，不把大文件全部放入 DOM。
- 引用、调用树、反向依赖、类型关系都显示加载/错误/未解析状态。
- 跳转使用路径 + 行号，不依赖易变的行号锚点作为长期身份。

## 6. 阅读能力

- 文件树、多标签页、历史前进/后退。
- 文件内搜索、全局文件/符号搜索。
- 符号大纲、容器分组、折叠、键盘导航。
- 定义跳转、语义引用。
- 调用者、调用树、文件依赖、反向依赖、依赖图。
- 跨版本当前文件 Diff 和符号级摘要。
- 类型继承/实现关系。
- 四套主题、80%–200% 缩放、会话恢复。

## 7. 本地翻译

```text
lattice.exe
  └─ 按需启动 lattice-translator.exe
       ├─ stdin: NDJSON 请求
       └─ stdout: NDJSON 结果
```

- 当前模型：OPUS-MT English → Chinese，INT8 ONNX。
- 请求带 `id`，只接受当前请求的结果。
- worker 代号防止旧进程退出影响新进程。
- 支持取消、截断提示、stderr 日志和错误状态。
- 模型查找顺序：`LATTICE_MODEL_DIR` → `%APPDATA%\Lattice\models` → exe 同级 `.content` → 当前目录。
- 模型不随主程序发布。

## 8. 数据与缓存

| 数据 | 位置 | 可重建 |
| --- | --- | --- |
| 源码包 | `.content/<pack>` | 需要重新获取 |
| 符号索引 | `<pack>/.lattice/symbol-index-v3.postcard` | 是 |
| 文档高亮缓存 | `<pack>/.lattice/documents/` | 是 |
| 会话配置 | `%APPDATA%\Lattice\session.json` | 可删除 |
| 翻译日志 | `%APPDATA%\Lattice\logs/translator.log` | 可删除 |
| 翻译模型 | `.content/models/` 或 APPDATA | 需要重新获取 |

不保存书签、笔记、指南或阅读进度。

## 9. 发布

- 应用：`lattice.exe`
- 翻译 worker：`lattice-translator.exe`
- 便携包：ZIP
- 安装器：Inno Setup
- Source Pack 和模型独立分发，不进入安装器
- 目标平台：Windows x64 + WebView2；Native/Blitz 仅为实验入口

## 10. 当前范围

已完成：多语言源码包、文件树/标签页、语义高亮、大纲、搜索、定义/引用、调用链、依赖图、反向依赖、跨版本 Diff、四套主题、缩放、会话恢复、本地翻译、类型继承/实现关系。

当前内容包：CPython、OpenJDK、Rust、TypeScript、SQLite（双版本）、Nginx、Spring Boot、Vue、React。

剩余：发布工程收尾（安装器、签名、CI、许可证清单）与体积复测。

不再扩展：笔记、指南、用户仓库索引、编辑器、课程、模型市场、更多语言对、自定义主题、多文件 Diff、i18n、自动更新、Native 后端功能对齐。
