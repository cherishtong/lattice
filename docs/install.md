# 安装与升级

## 系统要求

- Windows 10/11 x64
- WebView2 Runtime（Windows 11 通常已预装）
- 便携包约 12 MB；Source Pack 和翻译模型单独安装

## 便携版

1. 解压 `Lattice-<version>-windows-x64.zip`。
2. 运行 `lattice.exe`。
3. 把 Source Pack 解压到同目录的 `.content/<pack>/`，或在 `.content/` 下新建目录。
4. 翻译模型可选：运行 `scripts/fetch-opus-mt-model.ps1`，或放到
   `%APPDATA%\Lattice\models\opus-mt-en-zh`。

便携版不需要管理员权限，也不会写入注册表。删除目录即可卸载。

## 安装版

`Lattice-<version>-Setup.exe` 是 per-user 安装程序：

- 默认安装到 `%LOCALAPPDATA%\Programs\Lattice`
- 不需要管理员权限
- 创建开始菜单快捷方式和可选桌面快捷方式
- 卸载入口在“设置 → 应用 → Lattice”

静默安装：

```powershell
Lattice-0.1.0-Setup.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART
```

静默卸载：

```powershell
& "$env:LOCALAPPDATA\Programs\Lattice\unins000.exe" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART
```

## Source Pack

安装包不带源码包。推荐从仓库的 `source-packs/` 解压：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/install-source-packs.ps1
```

安装版用户把包解压到：

```text
%LOCALAPPDATA%\Programs\Lattice\.content\<pack>\
```

也可以用环境变量指定单个包或内容目录：

```powershell
$env:LATTICE_SOURCE_PACK = "D:\LatticePacks\openjdk-jdk-27"
lattice.exe
```

首次打开源码包会重建索引；之后读取 `<pack>/.lattice/` 缓存。缓存可删除，源码不受影响。

## 翻译模型

模型不随安装包发布：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/fetch-opus-mt-model.ps1
```

默认安装到 `.content/models/opus-mt-en-zh`；安装版用户可放到
`%APPDATA%\Lattice\models\opus-mt-en-zh`。

## 升级

- 便携版：解压新 ZIP 覆盖 `lattice.exe` 和 `lattice-translator.exe`，保留
  `.content/`。
- 安装版：直接运行新版本安装程序，安装目录和 Source Pack 保留。
- 索引缓存版本变化时会自动重建。

## 卸载

- 便携版：删除程序目录。
- 安装版：使用 Windows“应用和功能”卸载，或运行 `unins000.exe`。
- `.content/` 和 `%APPDATA%\Lattice` 属于用户数据，升级或卸载前建议自行备份。

## 代码签名

当前公开构建未签名，Windows SmartScreen 可能显示提示。正式分发如需签名，
在打包时设置 `LATTICE_SIGN_CERT_SHA1`，并确保 `signtool.exe` 可用。
