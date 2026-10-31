# Lattice 路线图

产品范围已经收敛：只做**离线源码阅读**、**深度关系理解**和**本地翻译**，不再扩展笔记、指南、主题导入、多模型或 IDE 类功能。

## 阶段状态

| 阶段 | 目标 | 状态 |
| --- | --- | --- |
| P0 UI 与桌面外壳 | 自绘标题栏、四套主题、WebView2、缩放、文件图标 | 完成 |
| P1 源码包阅读 | Source Pack、文件树、标签页、语义高亮、文件内大纲、搜索 | 完成 |
| P2 源码导航 | 全局搜索、定义跳转、引用、历史、快捷键 | 完成 |
| P3 多语言 | CPython、OpenJDK、Rust、TypeScript、SQLite、Nginx、Spring Boot、Vue、React | 完成 |
| P4 深度关系 | 调用链、调用树、依赖图、反向依赖、跨版本 Diff | 完成 |
| P5 类型关系 | 继承、接口、Trait 实现、C++ 基类、跨文件派生/实现者 | 完成 |
| P6 内容补齐 | Nginx 1.31.8、Spring Boot 4.1.2、Vue 3.5.44、React 19.3.1 源码包与回归 | 完成 |
| P7 本地翻译 | OPUS-MT 英译中、独立 worker、取消与错误处理 | 完成，保持单模型 |
| P8 发布工程 | 便携包、安装器、CI、许可证清单、校验和 | 完成（签名需证书） |

## 剩余功能

### P6 内容补齐

- [x] Nginx 1.31.8（固定 tag `release-1.31.8`）
- [x] Spring Boot 4.1.2（固定 tag `v4.1.2`）
- [x] Vue 3.5.44（固定 tag `v3.5.44`）
- [x] React 19.3.1（固定 tag `v19.3.1`）
- [x] 建立四个源码包的符号索引和文档缓存
- [x] 写入 manifest、checksum 和默认入口
- [x] 在真实源码上验证高亮、大纲、定义、引用、调用链、依赖和关系
- [x] 测量文件数、索引体积和首次索引时间

### P5 类型关系回归

- [x] Java：类 `extends`、接口 `implements`、接口多继承
- [x] JavaScript / TypeScript：类 `extends` / `implements`、接口 `extends`
- [x] C++：`class` / `struct` 基类列表、访问修饰符、模板类型
- [x] Rust：`impl Trait for Type`、`trait Foo: Bar + Baz`
- [x] 跨文件派生类型和实现者解析
- [x] 未解析目标明确显示为未解析

验证：Java `ArrayList.java`、TypeScript `checker.ts`、Rust `library/core/src/fmt/mod.rs`、Vue `renderer.ts`、React `ReactFiberWorkLoop.js` 与 Spring Boot `SpringApplication.java`。

### P8 发布工程

- [x] Inno Setup 实际构建与安装/卸载测试（per-user 安装）
- [x] 便携包和安装器 SHA-256
- [x] GitHub Actions：fmt、clippy、test、release build、package
- [x] 第三方许可证汇总
- [x] 更新体积与安装文档
- [ ] 代码签名：已接入可选 `LATTICE_SIGN_CERT_SHA1` 流程，公开发布默认未签名，需要 Windows 代码签名证书

## 明确移除

以下不再作为目标：

- 书签、笔记、阅读进度
- 外部 Markdown 指南导入与符号锚点
- 用户仓库索引、编辑、调试、IDE
- 内建课程与固定学习路径
- 模型市场、多模型管理、更多语言对、在线翻译
- 自定义主题导入
- 多文件/整包 Diff
- 正则/全词搜索、多窗口/分屏
- 界面国际化、自动更新
- Native/Blitz 后端功能对齐

## 后续原则

1. 先保证源码与关系的真实性，再做表现层增强。
2. 所有源码必须带有实现、版本、来源和许可证。
3. 解析不确定时明确降级，不伪造语义关系。
4. 阅读期间不访问网络；内容获取是显式操作。
5. 每个里程碑都必须断网验证。
