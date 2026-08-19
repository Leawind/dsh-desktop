| 中文 | [English](README.en.md) |
| ---- | ----------------------- |

<div align="center">

# DSH Desktop

[DeepSeek Harness (DSH)](https://github.com/deepseek-ai/deepseek-harness) 的轻量、跨平台桌面应用。

直接启动内置 DSH，连接已有服务，或使用系统、自定义路径和 npx 启动 DSH

[![最新发布](https://img.shields.io/github/v/release/Leawind/dsh-desktop?display_name=tag&sort=semver)](https://github.com/Leawind/dsh-desktop/releases)
![支持的平台](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-4c8bf5)
[![CI](https://github.com/Leawind/dsh-desktop/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Leawind/dsh-desktop/actions/workflows/ci.yml?query=branch%3Amain)
[![MIT 许可证](https://img.shields.io/github/license/Leawind/dsh-desktop)](https://github.com/Leawind/dsh-desktop/blob/main/LICENSE)

<img src="docs/images/main.png" alt="DSH Desktop 主工作区" width="80%">

</div>

---

## 为什么选择这个

- **跨平台**：支持 Windows、Linux 和 macOS，可从 [GitHub Releases] 下载、解压后运行
- **轻量分发**：`slim` 仅约 7-10 MB，适合已有 DSH 环境或只需连接服务的使用方式
- **自由选择 DSH 来源**：可使用内置运行时、系统 `dsh`、自定义可执行文件、npx，或直接连接已有 DSH 服务

## 选择适合你的版本

| 情况                                                      | 选择      | 安装包大小 | 说明                                          |
| --------------------------------------------------------- | --------- | ---------- | --------------------------------------------- |
| 希望下载后直接使用，或希望同时拥有内置和外部 DSH 启动方式 | `bundled` | 100-120 MB | 包含完整功能，并额外内置 Node.js、DSH 和 pnpm |
| 已安装 DSH，且不需要内置 DSH 运行时                       | `slim`    | 7-10 MB    | 保留除内置运行时外的全部桌面管理能力          |

## 安装与首次运行

从 Release 下载与平台对应的归档文件并解压后，都可以在应用的全局设置中选择以下 DSH 启动方式：Windows 和 macOS 提供 ZIP，Linux 提供 `tar.gz`。

- 使用 `bundled` 附带的内置运行时；
- 使用系统 `PATH` 中的 `dsh`；
- 选择自定义 DSH 可执行文件或启动脚本；
- 通过 npx 启动 `@deepseek-ai/dsh`；
- 输入已有 DSH Web 服务的地址。

`bundled` 默认使用内置运行时，因此无需预先安装 Node.js、npm、pnpm 或 `dsh`；也可以选择其余任一种方式。`slim` 不提供内置运行时，其余启动和连接方式与 `bundled` 相同。

如果选择系统 DSH，可通过 npm 安装：

```sh
npm install -g @deepseek-ai/dsh
```

通过 npx 启动时，系统需要安装 Node.js；首次运行会由 npm 下载所选的 DSH 版本。

Release 同时提供 `SHA256SUMS`，可用于校验下载文件的完整性。

## 界面概览

<div align="center">

<img src="docs/images/settings-dsh.png" width="94%">

选择 DSH 来源，并统一配置受管服务使用的 DSH Home。

---

<img src="docs/images/settings-startup.png" width="94%">

定义启动窗口时的行为。

---

<img src="docs/images/settings-runtime.png" width="94%">

查看和管理内置运行时。

---

</div>

DeepSeek Harness 仍处于开发预览阶段，安装和配置方式以其[官方文档](https://github.com/deepseek-ai/deepseek-harness/blob/master/README.zh.md)为准。

> [!IMPORTANT]
>
> 本项目由社区开发，并非 DeepSeek 官方项目，也未获得 DeepSeek 的官方认可或背书。

[GitHub Releases]: https://github.com/Leawind/dsh-desktop/releases
