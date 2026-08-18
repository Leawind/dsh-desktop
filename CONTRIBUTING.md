# 贡献指南

DSH Desktop 使用 pnpm workspace 管理 Vue 3 前端、UI 组件库与 Rust/WebUI Host。

## 开发环境

需要安装：

- Node.js 22.19.0 或更高版本；
- pnpm 11.21.0；
- Rust stable 工具链，并安装 `rustfmt`；
- `make`、C/C++ 编译器和 WebUI 在当前平台需要的系统库；
- 已安装的 Chromium、Chrome、Firefox 或其他 WebUI 支持的浏览器；
- 运行默认 `slim` 变体时可用的 `dsh` 命令。

Debian 或 Ubuntu 可安装基础构建工具：

```sh
sudo apt install build-essential curl wget file
```

## 仓库结构

```text
apps/
├── desktop/   WebUI 配置、Rust Host 与桌面应用构建入口
└── frontend/  Vue 3 应用界面与 Vite 构建入口
packages/
└── ui/        可复用的 Vue 组件与设计 token
```

## 开发

安装依赖后启动默认 `slim` 开发版本：

```sh
pnpm install
pnpm dev
```

该命令启动 Vite 与 Rust Host。浏览器窗口加载 Vite 页面，`/api` 由 Vite 代理到 Host，因此 Vue、CSS 与国际化资源的修改会通过 Vite 热更新立即显示。再次执行该命令时，会复用已有 Vite 与 Host，并由 Host 新建浏览器窗口。最后一个窗口关闭后，Host 与由该命令启动的 Vite 开发进程会一同退出。

开发 `bundled` 变体：

```sh
pnpm dev:bundled
```

首次运行会准备锁定版本的 Node.js、DSH 与 pnpm 运行时。应用默认连接 `http://127.0.0.1:3080`；也可以在设置中选择系统、`npx` 或自定义 DSH 来源，以及自定义 DSH Home。

## 命令

| 命令                   | 用途                                     |
| ---------------------- | ---------------------------------------- |
| `pnpm dev`             | 启动 Vite 热更新与 `slim` Host           |
| `pnpm dev:bundled`     | 启动 Vite 热更新与 `bundled` Host        |
| `pnpm build:bundled`   | 构建包含内置运行时的单文件可执行程序     |
| `pnpm build:slim`      | 构建不含内置运行时的单文件可执行程序     |
| `pnpm build`           | 依次构建当前平台的两种发行变体           |
| `pnpm runtime:prepare` | 准备并校验当前平台的内置运行时           |
| `pnpm check`           | 运行格式、静态、类型、测试和生产构建检查 |
| `pnpm format`          | 格式化源码与配置                         |
| `pnpm frontend:dev`    | 单独启动 Vite；需要另行启动 Host         |

提交前至少运行：

```sh
pnpm check
```

## 构建可执行文件

```sh
pnpm build
```

构建脚本遵循 Cargo 当前的 target 目录配置。在未自定义的情况下，产物位于 Cargo 默认 target 目录的：

```text
release/artifacts/
```

文件名为 `dsh-desktop-<version>-<variant>-<platform>-<architecture>`；Windows 文件带 `.exe` 扩展名。每个文件可独立移动和运行。Linux 从浏览器下载后可能需要先赋予执行权限。

不同操作系统的可执行文件应在对应平台构建。推送 `v<version>` 标签后，GitHub Actions 会构建 Linux、Windows 与 macOS 的 `bundled`、`slim` 可执行文件并发布到同一 GitHub Release。

## 内置运行时

内置运行时版本记录在 `runtime/versions.json`，生产依赖闭包由 `runtime/package-lock.json` 固定。准备脚本校验 Node.js 官方摘要、npm 包 integrity、关键入口文件与第三方许可证，并生成 `payload.tar.gz`。该 payload 在发行构建时嵌入 `bundled` 可执行文件，首次使用时由 Host 解压并再次校验。

## 发布版本

应用版本以 `apps/desktop/Cargo.toml` 为唯一来源。准备新版本：

```sh
pnpm run release:version -- 0.2.0
pnpm check
pnpm run release:prepare -- v0.2.0 release-notes.md
```

填写更新日志、提交后创建与版本一致的附注标签 `v0.2.0`。仅在明确需要时推送提交与标签。

## 前端约定

- 前端使用 Vue 3 和 TypeScript；
- 用户可见文本同时提供 `zh-CN` 与 `en-US`；
- 通用 UI 能力放在 `packages/ui`；
- 应用业务逻辑、HTTP bridge 和页面放在 `apps/frontend`。
