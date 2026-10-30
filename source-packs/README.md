# Source Pack 归档

本目录保存可直接解压的 Source Pack 归档。归档只包含源码、许可证和 `.lattice/pack.json`，不包含符号索引、文档缓存或损坏隔离目录。

## 归档

| 归档 | 实现 / 版本 | 大小 |
| --- | --- | ---: |
| `cpython-3.13.2.zip` | CPython 3.13.2 | 30.2 MiB |
| `openjdk-jdk-27.zip` | OpenJDK 27（`java.base`） | 12.1 MiB |
| `rust-1.99.1.zip` | Rust 1.99.1 | 38.9 MiB |
| `typescript-7.0.2.zip` | TypeScript 7.0.2 | 5.6 MiB |
| `sqlite-3.54.1.zip` | SQLite 3.54.1 | 14.0 MiB |
| `sqlite-3.50.4.zip` | SQLite 3.50.4 | 13.6 MiB |
| `nginx-1.31.8.zip` | Nginx 1.31.8 | 1.3 MiB |
| `spring-boot-4.1.2.zip` | Spring Boot 4.1.2 | 4.4 MiB |
| `vue-3.5.44.zip` | Vue 3.5.44 | 0.5 MiB |
| `react-19.3.1.zip` | React 19.3.1 | 2.1 MiB |

合计约 122.7 MiB。

## 安装

在仓库根目录运行：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/install-source-packs.ps1
```

只安装部分：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/install-source-packs.ps1 -Pack cpython-3.13.2,rust-1.99.1
```

覆盖已安装目录：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/install-source-packs.ps1 -Pack nginx-1.31.8 -Force
```

归档会解压到 `.content/<pack>/`。首次打开源码包时重建索引，之后使用本地缓存。

## 校验和

```text
ba0725ed80fc445c238e8420dc3c1474f4e2f67a8e328e550038b0a18f391865  cpython-3.13.2.zip
6d160046911441bbe84b81242fececb1ef7ebe21a44e20386c25a442713e7b99  openjdk-jdk-27.zip
6e01d792d9f90726b625e6be907341ba6dd37e8ffec521f2684f5baf2e067f96  rust-1.99.1.zip
e459e5109f33cd6bd14a612241814e59dad8ec6852bec9290b59ec713157e5ee  typescript-7.0.2.zip
c2d35bdba2594142aeb80feca72840826cc6476bd3fa1e7c23542960fc4b31e5  sqlite-3.54.1.zip
cc0959e500cec8b285ea77d8aed6af6d6e72c9717bf2c022a9968323f35872c0  sqlite-3.50.4.zip
ee965926db198c5debdd330be74d36b2863eca7716538a9485e1ab9d2e98d11b  nginx-1.31.8.zip
a98932b340c14446937002869c7f89124aff427efb62b0b5477698f5f9632082  spring-boot-4.1.2.zip
545b2576755a3bffc3a0cfeb7fe9aac55e6d3fb02ef430f71aa2154855ea2c86  vue-3.5.44.zip
7c1ec20aab759c7a5979a85e18788abf81b0c2e26e256ed958392b34d0fec62e  react-19.3.1.zip
```

## 重新生成

```powershell
powershell -ExecutionPolicy Bypass -File scripts/package-source-packs.ps1
```

脚本从 `.content/` 读取源码包，排除 `.lattice/documents`、`symbol-index-*.postcard` 和 `.lattice/src-broken`，输出到本目录，并检查单个归档不超过 GitHub 的 100 MB 文件限制。
