# Source Pack：内置源码包

## 当前内容

| 实现 | 语言 | 源码范围 | 本地目录 | 规模 |
| --- | --- | --- | --- | --- |
| CPython 3.13.2 | Python 实现 / C 源码 | 完整 CPython 源码 | `.content/cpython-3.13.2` | 4,885 文件 / 约 102 MiB |
| OpenJDK 27 | Java | `java.base/share/classes` | `.content/openjdk-jdk-27` | 3,172 文件 / 约 45 MiB |
| Rust 1.99.1 | Rust | `compiler/`、`library/`、`src/` | `.content/rust-1.99.1` | 18,386 文件 / 约 160 MiB |
| TypeScript 7.0.2 | TypeScript | `src/`（编译器、服务、标准库声明） | `.content/typescript-7.0.2` | 748 文件 / 约 34 MiB |
| SQLite 3.54.1 | C | 官方完整源码树 | `.content/sqlite-3.54.1` | 2,287 文件 / 约 108 MiB |
| SQLite 3.50.4 | C | 官方完整源码树 | `.content/sqlite-3.50.4` | 2,212 文件 / 约 106 MiB |
| GCC 15.1.0 | C / C++ | `gcc/`、`libgcc/`、`libstdc++-v3/`、`include/` | `.content/gcc-15.1.0` | 通过脚本获取 |
| Clang 20.1.0 | C++ | `clang/`、`llvm/include/`、`llvm/lib/Support/` | `.content/clang-20.1.0` | 通过脚本获取 |

获取脚本：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/fetch-cpython-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-openjdk-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-rust-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-typescript-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-sqlite-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-gcc-pack.ps1
powershell -ExecutionPolicy Bypass -File scripts/fetch-clang-pack.ps1
```

源码包不进入 Git，只在本地准备。应用从 `.content` 自动发现源码包，也可以用 `LATTICE_SOURCE_PACK` 指向其他目录。

产品不强调版本管理和许可证展示，源码包版本只作为缓存失效与来源标识，不在主界面呈现。

## 支持的语言

- C：tree-sitter C，高亮、函数/结构体/枚举/宏大纲。
- C++：tree-sitter C++，高亮、命名空间/类/结构体/方法/函数/类型/宏大纲。
- Java：tree-sitter Java，高亮、类/接口/枚举/记录/方法/构造器大纲。
- Rust：tree-sitter Rust，高亮、函数/方法/结构体/枚举/Trait/模块/宏/常量大纲。
- TypeScript：tree-sitter TypeScript/TSX，高亮、类/接口/枚举/类型别名/函数/方法/构造器/模块大纲。

其他 UTF-8 文件按纯文本显示。

## 安全边界

- 应用只读源码，不写入包内文件。
- 所有读取路径必须是包内相对路径，拒绝绝对路径、父目录跳转和符号链接。
- 单个文件超过 4 MiB 时当前拒绝打开，避免一次性文本模型失控。
- 文件扫描、UTF-8 解码和 tree-sitter 解析都在后台任务执行。

## 阅读能力

- 文件树按目录优先、名称不区分大小写排序。
- 忽略以 `.` 开头的目录，避免 `.git` 等元数据。
- `.c` / `.h` 使用 tree-sitter C；`.cc` / `.cpp` / `.cxx` / `.hpp` / `.hh` / `.hxx` 使用 tree-sitter C++；`.java`、`.rs`、`.ts` / `.tsx` 分别使用对应语法。
- 文件内大纲识别 C、C++、Java、Rust、TypeScript 的主要声明；Java、TypeScript、Rust、C++ 方法按所属容器嵌套展示。
- 代码支持按符号折叠、全部折叠/展开和折叠状态持久化；引用扫描可取消、分页加载，并可隐藏条件编译分支。
- 其他 UTF-8 文本目前按纯文本显示。

## 导入与卸载

- 书库底部“导入源码包”接受本地目录或 `.zip` 路径；目录内需有 `.lattice/pack.json`。
- ZIP 导入会拒绝绝对路径与 `..` 穿越，并校验每个条目的 CRC。
- 导入过程显示复制/解压与符号索引阶段，可随时取消；索引完成后可直接搜索，文档渲染缓存按需生成。
- 导入时复制到最近的 `.content`，保留 `pack.json`，跳过旧的 `.lattice` 缓存与 `.git`。
- 清单可带 `checksum` 字段；导入时校验 CRC32 内容校验和，不匹配则拒绝安装。
- 已存在同名包时拒绝覆盖。
- 卸载只对非当前包开放，需要二次确认；实现只会删除 `.content` 内的目录。

## 源码包清单与书库

每个源码包通过 `.lattice/pack.json` 声明元数据：

```json
{
  "id": "cpython-3.13.2",
  "name": "CPython",
  "implementation": "CPython",
  "version": "3.13.2",
  "language": "Python",
  "source_language": "C",
  "license": "PSF-2.0",
  "default_entry": "Objects/listobject.c",
  "checksum": "crc32:838f6da2"
}
```

- 应用从 `.content` 发现所有有效源码包。
- 顶部“本地源码书库”按名称展示实现语言、源码语言与路径；只允许只读阅读。
- 当前版本可即时切换；切换会重建标签页、符号索引和阅读状态。
- `LATTICE_SOURCE_PACK` 可以指向 `.content` 之外的源码包目录。
- 书库支持从本地目录导入源码包，并可二次确认后卸载。
- 当前包、标签页、活动文件、滚动位置、折叠状态、最近搜索、主题、界面缩放与窗口位置/大小保存在 `%APPDATA%\Lattice\session.json`（可用 `LATTICE_CONFIG_DIR` 覆盖目录），不写入源码包。

## 符号索引缓存

全局文件/符号搜索使用预构建索引：

- 位置：`<pack>/.lattice/symbol-index-v2.postcard`
- 格式：Postcard 二进制
- 指纹：源码包名称、版本、源码文件路径与文件大小
- 首次运行或指纹变化时后台构建并写回
- 后续 release 启动可直接加载，不再重复解析全部源码
- 文件打开不再与全量索引构建争抢 CPU

当前预构建缓存：

| 源码包 | 符号 | 索引大小 | 文档缓存 | 文档数 |
| --- | ---: | ---: | ---: | ---: |
| CPython 3.13.2 | 38,339 | 3.12 MiB | 9.11 MiB | 1,027 |
| OpenJDK 27 | 57,521 | 7.09 MiB | 9.03 MiB | 3,108 |
| Rust 1.99.1 | 220,047 | 22.88 MiB | 按需补齐 | 3,966（v2） |
| TypeScript 7.0.2 | 21,051 | 2.51 MiB | 2.88 MiB | 709 |
| SQLite 3.54.1 | 19,227 | 1.71 MiB | 4.62 MiB | 359 |

## 文档渲染缓存

符号索引不包含逐行语法高亮结果，因此另外保存文档缓存：

- 位置：`<pack>/.lattice/documents/*.postcard`
- 单文件指纹基于完整源码内容
- 打开文件时读取缓存、恢复行文本与高亮 span，不运行 tree-sitter
- 缓存缺失或源码变化时回退到解析，并自动写回
- CPython 全量校验 1,027 个缓存 release 下约 1.1 秒（仅预构建检查，不是打开耗时）

正式打包 Source Pack 时应同时携带符号索引和文档缓存；`.content` 在开发环境中不进入 Git。可手动预构建：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/prepare-cpython-index.ps1 -Pack .content/rust-1.99.1
```

## 下一步

1. 调用链与模块依赖图。
2. 同一实现跨版本的源码 Diff。
3. 正式发布签名与自动更新。
