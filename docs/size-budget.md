# Windows 体积基线

测量日期：2026-10-31
平台：Windows x64，`x86_64-pc-windows-msvc`  
构建：`cargo build --release`

## 当前产物

| 文件 | 大小 | 说明 |
| --- | ---: | --- |
| `lattice.exe` | 19.1 MB | WebView2 主程序 |
| `lattice-translator.exe` | 24.4 MB | ONNX Runtime + OPUS-MT worker |
| `Lattice-0.1.0-windows-x64.zip` | 12.7 MB | 便携包（两个 EXE + 文档 + 许可证） |
| `Lattice-0.1.0-Setup.exe` | 9.7 MB | per-user 安装器（LZMA2） |
| `opus-mt-en-zh` 模型 | 116.8 MB | 单独下载，不进入安装包 |

## 运行依赖

- **WebView2 Runtime**：Windows 11 通常已预装；完全离线分发需单独预算约 205 MB 的 Evergreen Standalone 安装器。
- **ONNX Runtime**：由 `ort` 构建链链接；翻译 worker 不依赖系统安装的 ONNX Runtime。
- **Source Pack**：CPython、OpenJDK、Rust、TypeScript、SQLite、Nginx、Spring Boot、Vue、React 均单独安装，不进入应用安装包。

## 分发口径

| 方案 | 应用 | 运行时 | 模型 | 源码包 |
| --- | ---: | ---: | ---: | --- |
| WebView2，系统已有运行时 | 12.7 MB ZIP | 0 | 116.8 MB（可选） | 按需 |
| WebView2，完全离线 | 12.7 MB ZIP | 约 205 MB | 116.8 MB（可选） | 按需 |
| 只装主程序，不翻译 | 12.7 MB ZIP | 0 | 0 | 按需 |

## 说明

- 正式安装器使用 LZMA 压缩，实际载荷可能略小于 ZIP。
- 代码签名、安装器框架和图标通常增加不到 1–2 MB。
- Native/Blitz 实验产物不进入正式发布基线。
- Source Pack 和翻译模型体积远大于应用本体，必须独立分发。

复测：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/package-release.ps1
```
