# Windows 安装包体积基线

测量日期：2026-10-25  
平台：Windows x64，`x86_64-pc-windows-msvc`  
构建：`cargo build --release`（默认 release profile，尚未做 LTO/体积特化）  
压缩：ZIP / Deflate，仅包含应用可执行文件

## 实测结果

| 分发方式 | 裸 EXE | ZIP 包 | 额外系统依赖 |
| --- | ---: | ---: | --- |
| Dioxus WebView2 | 14,517,248 B / **13.84 MiB** | 3,504,862 B / **3.34 MiB** | WebView2 Runtime |
| Dioxus Native / Blitz | 28,111,360 B / **26.81 MiB** | 10,198,579 B / **9.73 MiB** | 无 WebView；使用系统 GPU / DirectX 驱动 |

WebView2 行在接入 C/Java/Rust/TypeScript 四语法、完整文件图标库、会话恢复与源码包管理后重测；Native 行沿用 2026-10-24 的数据。

应用资源（CSS、图标、演示源码）已编译进可执行文件，没有单独的资源目录。

## 关键差异：WebView2 运行时

WebView2 应用不是完整的离线独立安装包。目标机器必须已有 WebView2 Runtime。

Windows 11 通常随 Edge 预装 WebView2。此时 WebView 版本只需要约 **3.34 MiB** 的 ZIP，明显小于 Native。

但面向不确定的 Windows 10/11 离线环境时，必须准备微软的 **Evergreen Standalone Installer（x64）**：

- 测量日期：2026-10-24
- 官方链接：<https://go.microsoft.com/fwlink/?linkid=2124701>
- 文件：`MicrosoftEdgeWebView2RuntimeInstallerX64.exe`
- HTTP `Content-Length`：**214,823,632 B / 204.87 MiB**
- 微软文档称 Fixed Version 二进制超过 250 MB

因此完整离线分发的近似预算：

| 方案 | 应用 | 运行时 | 合计 |
| --- | ---: | ---: | ---: |
| WebView2，目标机器已有运行时 | 3.34 MiB ZIP | 0 | **约 3.34 MiB** |
| WebView2，完全离线携带 Evergreen | 3.34 MiB ZIP | 214.82 MB | **约 206 MiB 压缩载荷** |
| Native，完全离线 | 9.73 MiB ZIP | 0 | **约 9.73 MiB** |

注：

- WebView2 Standalone 安装器本身已经是压缩分发格式，再次打进安装器不一定能明显缩小。
- 正式 NSIS/MSI 使用 LZMA 等压缩，应用载荷可能比 ZIP 略小；上面的 ZIP 是保守口径。
- 安装器框架、代码签名和图标通常增加不到 1–2 MB。
- 这里不含 Source Pack。CPython、OpenJDK 等源码包必须与安装器分离，否则内容体积会远超应用本身。

## 结论

### 如果只承诺 Windows 11，或允许首次联网安装运行时

选择 **WebView2**：

- 应用体积约为 Native 的 1/5。
- UI 能力和 CSS 一致性更成熟。
- 安装包可维持 2 MB 级。

### 如果“离线优先”包含完全离线的 Windows 10/11 安装

选择 **Native**：

- 不需要 WebView2，真正单文件运行。
- 离线包约为 10 MiB，而 WebView2 方案约 206 MiB。
- 这比应用本体 5 MiB 对 24 MiB 的差距重要两个数量级。

### 建议的产品分发策略

1. **Native 作为离线版主安装包。** 符合“下载一次，之后完全离线”的产品定位。
2. **WebView2 作为轻量版/在线安装包。** 面向 Windows 11 或已安装运行时的用户。
3. 后续可评估“双可执行文件”安装器：WebView2 与 Native 共用同一套 RSX，安装器检测运行时后选择启动项；应用载荷仍约 10.5 MiB 压缩体积。
4. 无论选哪种，Source Pack 都应该按需安装，不进入基础安装器。

## 复测

```powershell
powershell -ExecutionPolicy Bypass -File scripts/measure-size.ps1
```

结果写入 `dist/sizes.json`，并生成两个可独立运行的 EXE/ZIP。`dist/` 与 `.tools/` 不进入 Git。
