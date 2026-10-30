# Source Pack

Source Pack 是 Lattice 的不可变源码单元：固定实现、版本、源码范围和许可证。源码包不进入 Git，安装后只读。

## 内置内容

| 实现 | 版本 | 源码范围 | 本地目录 | 规模 |
| --- | --- | --- | --- | --- |
| CPython | 3.13.2 | 完整 CPython 源码 | `.content/cpython-3.13.2` | 4,885 文件 |
| OpenJDK | 27 | `java.base/share/classes` | `.content/openjdk-jdk-27` | 3,172 文件 |
| Rust | 1.99.1 | `compiler/`、`library/`、`src/` | `.content/rust-1.99.1` | 18,237 文件 |
| TypeScript | 7.0.2 | 编译器、服务、标准库声明 | `.content/typescript-7.0.2` | 747 文件 |
| SQLite | 3.54.1 | 官方完整源码树 | `.content/sqlite-3.54.1` | 2,287 文件 |
| SQLite | 3.50.4 | 官方完整源码树 | `.content/sqlite-3.50.4` | 2,208 文件 |
| Nginx | 1.31.8 | `src/` | `.content/nginx-1.31.8` | 425 文件 / 6.5 MiB |
| Spring Boot | 4.1.2 | `core/`、`module/` 的 `src/main/java` | `.content/spring-boot-4.1.2` | 3,308 文件 / 8.7 MiB |
| Vue | 3.5.44 | `packages/*/src` | `.content/vue-3.5.44` | 236 文件 / 1.6 MiB |
| React | 19.3.1 | 核心包的 `packages/*/src` | `.content/react-19.3.1` | 765 文件 / 9.7 MiB |

所有源码包必须在书库、侧栏、文件头、文件信息和状态栏显示版本。

## 获取

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

Nginx、Spring Boot、Vue、React 使用固定 tag，不跟随分支头；下载优先走镜像，失败后回退 GitHub。Spring Boot 只保留框架主体模块，React 只保留核心运行时相关的包。

## 支持的语言

- C：tree-sitter C，高亮、函数/结构体/枚举/宏大纲。
- C++：tree-sitter C++，高亮、命名空间/类/结构体/方法/函数/类型/宏大纲。
- Java：tree-sitter Java，高亮、类/接口/枚举/记录/方法/构造器大纲。
- JavaScript：tree-sitter JavaScript，高亮、类/方法/函数/箭头函数大纲，支持 JSX。
- Rust：tree-sitter Rust，高亮、函数/方法/结构体/枚举/Trait/模块/`impl`/常量大纲。
- TypeScript：tree-sitter TypeScript/TSX，高亮、类/接口/枚举/类型别名/函数/方法/构造器/模块大纲。
- 其他 UTF-8 文件按纯文本显示。

## 安全边界

- 只读源码，不写入包内文件。
- 所有读取路径必须是包内相对路径，拒绝绝对路径、父目录跳转和符号链接。
- 单个文件超过 4 MiB 时拒绝打开。
- 文件扫描、UTF-8 解码和 tree-sitter 解析都在后台任务执行。

## 导入与卸载

- 书库底部“导入源码包”接受目录或 ZIP；目录内需有 `.lattice/pack.json`。
- ZIP 导入拒绝绝对路径与 `..` 穿越，并校验每个条目 CRC。
- 导入显示复制/解压与符号索引阶段，可取消；取消后清理半成品。
- 清单可带 `checksum` 字段；导入时校验内容 CRC32，不匹配则拒绝安装。
- 已存在同名包时拒绝覆盖。
- 卸载只对非当前包开放，需要二次确认；只删除 `.content` 内的目标目录。

## 索引与缓存

- 符号索引：`<pack>/.lattice/symbol-index-v3.postcard`
- 文档高亮缓存：`<pack>/.lattice/documents/*.postcard`
- 索引包含文件、符号和类型关系。
- 缓存缺失、版本变化或源码指纹变化时自动重建。
- release 构建优先加载缓存，不重复解析全部源码。

预构建：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/prepare-cpython-index.ps1 -Pack .content/nginx-1.31.8
```

## 当前索引规模

| 源码包 | 符号数 | 类型关系 | 索引体积 | 文档缓存 |
| --- | ---: | ---: | ---: | ---: |
| CPython 3.13.2 | 38,479 | 8 | 3.15 MiB | 1,027 文件 / 8.93 MiB |
| OpenJDK 27 | 57,448 | 3,381 | 7.41 MiB | 3,108 文件 / 9.14 MiB |
| Rust 1.99.1 | 219,494 | 16,806 | 26.65 MiB | 11,773 文件 / 32.67 MiB |
| TypeScript 7.0.2 | 21,051 | 2,312 | 2.82 MiB | 709 文件 / 3.14 MiB |
| SQLite 3.54.1 | 19,227 | 0 | 1.63 MiB | 359 文件 / 4.62 MiB |
| SQLite 3.50.4 | 19,340 | 0 | 1.64 MiB | 390 文件 / 4.51 MiB |
| Nginx 1.31.8 | 7,546 | 0 | 0.66 MiB | 2.10 MiB |
| Spring Boot 4.1.2 | 23,352 | 1,739 | 5.03 MiB | 3.09 MiB |
| Vue 3.5.44 | 2,209 | 141 | 0.24 MiB | 0.26 MiB |
| React 19.3.1 | 9,654 | 1,059 | 1.16 MiB | 2.37 MiB |

GCC / Clang 不再作为内置内容，相关获取脚本和文档已移除。

## 范围

源码包只服务于源码阅读、搜索、导航、关系分析和版本对比。不包含笔记、指南、课程或用户仓库内容。
