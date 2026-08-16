# 贡献指南

感谢你参与 DSH Desktop 的开发。本项目使用 pnpm workspace 管理 Vue 3 前端、UI 组件库和 Tauri 桌面端。

## 开发环境

需要安装：

- Node.js 22.19.0 或更高版本；
- pnpm 11.21.0；
- Rust stable 工具链，并安装 `rustfmt`；
- [Tauri 2 对应平台的系统依赖](https://v2.tauri.app/start/prerequisites/)；
- 运行默认 `slim` 开发版本时需要可用的 `dsh` 命令。

Debian 或 Ubuntu 可以使用以下命令安装 Tauri 的 Linux 开发依赖：

```sh
sudo apt install libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

如果 `dsh` 不在桌面应用可见的 `PATH` 中，可以在 DSH Desktop 的全局设置中填写其绝对路径。

## 仓库结构

```text
apps/
├── desktop/   Tauri 配置、Rust Host 与桌面应用构建入口
└── frontend/  Vue 3 应用界面与 Vite 构建入口
packages/
└── ui/        可复用的 Vue 组件与设计 token
```

根目录的 `package.json` 和 `pnpm-workspace.yaml` 负责工作区编排。本应用专用的界面和业务逻辑位于 `apps/frontend`；不包含 DSH Desktop 业务逻辑的通用组件、动画和设计 token 位于 `packages/ui`。

## 启动开发版本

安装依赖：

```sh
pnpm install
```

启动 Vite 和 Tauri 开发进程：

```sh
pnpm run dev
```

应用默认连接 `http://127.0.0.1:3080`。启动前不需要手动运行 `dsh web`：应用会复用该地址上已有的 DSH 服务，或者通过系统中的 `dsh` 命令启动服务。受管服务默认继承开发进程的 `DSH_HOME`，环境变量未设置时由 DSH 使用 `~/.dsh`；也可以在全局设置中指定自定义目录。

要使用项目准备的独立 Node.js、DSH 和 pnpm 开发 `bundled` 变体，请运行：

```sh
pnpm run dev:bundled
```

第一次执行会下载并准备锁定版本的运行时；之后在运行时定义没有变化时会直接复用已有目录。

## 网络与代理

DSH Desktop 自身的 WebView 和 DSH 可达性探测会直接连接目标地址，本地开发不依赖系统代理中的 localhost 忽略规则。

由应用启动的 DSH 子进程仍会继承原有代理环境，因此 DSH 访问模型服务时可以继续使用用户配置的代理。实现方式和平台边界见[网络与代理策略](./docs/design.md#网络与代理策略)。

## 项目命令

所有供本地开发和 CI 使用的命令都定义在根目录的 `package.json` 中。

| 命令                             | 用途                                                         |
| -------------------------------- | ------------------------------------------------------------ |
| `pnpm run dev`                   | 启动完整的 Tauri 开发版本                                    |
| `pnpm run dev:bundled`           | 使用项目内置运行时启动 `bundled` 开发版本                    |
| `pnpm run dev:slim`              | 使用系统或自定义 DSH 启动 `slim` 开发版本                    |
| `pnpm run build:bundled`         | 构建包含 Node.js、DSH 和 pnpm 的安装包                       |
| `pnpm run build:slim`            | 构建不包含 DSH 运行环境的小型安装包                          |
| `pnpm run build`                 | 依次构建当前平台的两种安装包                                 |
| `pnpm run runtime:prepare`       | 准备并校验当前平台的内置运行时                               |
| `pnpm run runtime:test`          | 运行构建和发布脚本的测试                                     |
| `pnpm run release:prepare`       | 校验发布版本并从更新日志生成本次发布说明                     |
| `pnpm run release:version`       | 设置应用版本并创建对应的更新日志章节                         |
| `pnpm run check`                 | 依次运行格式检查、前端 lint 和测试、前端生产构建及 Rust 测试 |
| `pnpm run format`                | 格式化前端、工作区配置和 Rust 代码                           |
| `pnpm run format-check`          | 检查前端、工作区配置和 Rust 代码格式                         |
| `pnpm run frontend:dev`          | 单独启动 Vite 开发服务器                                     |
| `pnpm run frontend:build`        | 执行应用前端类型检查并构建生产资源                           |
| `pnpm run frontend:format`       | 格式化应用前端和 UI 包                                       |
| `pnpm run frontend:format-check` | 检查应用前端和 UI 包的格式                                   |
| `pnpm run frontend:lint`         | 检查应用前端和 UI 包                                         |
| `pnpm run frontend:test`         | 运行前端测试                                                 |
| `pnpm run frontend:typecheck`    | 检查应用前端和 UI 包的 TypeScript 类型                       |
| `pnpm run rust:format`           | 格式化 Rust 代码                                             |
| `pnpm run rust:format-check`     | 检查 Rust 代码格式                                           |
| `pnpm run rust:test`             | 运行 Rust 测试                                               |

提交更改前至少运行：

```sh
pnpm run check
```

如果修改了 UI 包，还应显式运行完整的前端类型检查：

```sh
pnpm run frontend:typecheck
```

## 构建安装包

```sh
pnpm run build:slim
pnpm run build
```

Tauri 会在 Cargo target 目录的 `release/bundle/installers/` 下汇总当前操作系统支持的安装文件。发布文件统一采用 `dsh-desktop-<version>-<variant>-<platform>-<architecture>.<extension>` 格式。未自定义 Cargo target 目录时，默认位置为：

```text
apps/desktop/target/release/bundle/installers/
```

设置了 `CARGO_TARGET_DIR` 或 Cargo 全局 target 目录时，产物会写入对应目录。

- Linux 构建 DEB、RPM 和 AppImage；
- Windows 构建 Windows 安装包；
- macOS 构建 macOS 应用和安装镜像。

不同操作系统的安装包需要在对应平台上构建。

排查单一格式时，可以将 Tauri 的 bundle target 交给统一构建入口，例如：

```sh
pnpm run build:bundled -- --bundles appimage
```

### 内置运行时准备

内置运行时版本记录在 `runtime/versions.json`，生产依赖闭包由 `runtime/package-lock.json` 固定。准备脚本完成以下工作：

- 从 Node.js 官方发布目录下载当前平台和架构的归档，并按官方 `SHASUMS256.txt` 校验；
- 从 npm 官方注册表按 lockfile 安装 DSH 和 pnpm；
- 只运行 `runtime/package.json` 中明确允许的依赖安装脚本；
- 为 GNU/Linux 目标移除依赖包中仅供 musl 使用的备用原生模块；
- 验证 Node.js、DSH 和 pnpm 版本，生成运行时文件摘要和第三方包清单；
- 将 payload 写入经校验的 `payload.tar.gz`，供 Host 首次启动时解包。

`curl` 和 npm 会正常使用环境中的代理设置。Node.js 归档下载较慢时，可以手动下载脚本输出的 URL，再指定本地文件：

```sh
pnpm run runtime:prepare -- --node-archive /path/to/node-archive
```

准备结果位于 `apps/desktop/runtime/bundled/`，属于本机构建产物，不提交到 Git。

### AppImage 构建运行时

Linux 构建脚本会为 `appimagetool` 准备当前架构的 AppImage runtime，按
`scripts/appimage-runtimes.json` 中固定的 SHA-256 校验后缓存到
`.cache/build-tools/appimage/`。`bundled` 与 `slim` 共用该缓存，后续构建不再重复下载。
构建入口同时禁止 `linuxdeploy` strip AppDir 中的 ELF；内置运行时位于压缩归档中，不会被
AppImage 工具扫描或改写。

`bundled` RPM 不再对已经压缩的 payload 重复压缩；`slim` RPM 使用 Tauri 默认压缩设置。

首次下载会继承 `curl` 支持的代理环境。如果自动下载较慢，也可以手动下载构建日志中的
`runtime-x86_64` 或 `runtime-aarch64`，再将本地文件传给任一构建命令：

```sh
pnpm run build:slim -- --appimage-runtime /path/to/runtime-x86_64
```

本地文件同样必须通过项目固定的校验值。上游 runtime 发生变化时，应明确审查并更新 URL
和校验值，而不是跳过校验。

## 发布版本

推送以 `v` 开头的 tag 会触发 GitHub Actions 构建 Linux、Windows 和 macOS 安装包，并将各平台的 `bundled` 与 `slim` 产物发布到同一个 GitHub Release。

应用版本以 `apps/desktop/Cargo.toml` 为唯一来源，Tauri 构建和应用内元数据会直接使用该版本。准备新版本时运行：

```sh
pnpm run release:version -- 0.2.0
```

该命令会更新 Cargo 包版本、刷新 `Cargo.lock`，并在 `CHANGELOG.md` 顶部创建对应的 `## [0.2.0]` 小节。填写该小节后，提交全部版本与更新日志改动。

tag 必须采用 `v<版本号>` 格式，例如 `v0.2.0`，并与 Cargo 包版本及更新日志章节一致。对应更新日志小节的内容会成为本次 GitHub Release 的发布说明；仅保留命令生成的占位注释无法通过发布检查。

推送 tag 前可以在本地检查发布元数据并预览生成的发布说明：

```sh
pnpm run release:prepare -- v0.2.0 release-notes.md
```

生成的 `release-notes.md` 仅用于预览，不需要提交。

## 前端约定

- 应用前端使用 Vue 3 和 TypeScript；
- 不直接编写 JavaScript 源文件；
- 用户可见文本使用 `vue-i18n`，当前语言资源为 `zh-CN` 和 `en-US`；
- 新增语言时必须保持翻译键和插值参数完整，并保留 `zh-CN` 作为回退语言；
- 通用 UI 能力优先实现于 `packages/ui`，并尽量与 DSH 的尺寸、间距、状态和动画保持一致；
- 应用业务逻辑、Tauri bridge 和具体页面保留在 `apps/frontend`。

详细的前端边界和视觉规范见[设计文档](./docs/design.md#前端实现)。

## 提交与拉取请求

- 提交信息使用英语；
- 使用 Conventional Commits 风格，例如 `feat: add service status view`、`fix: bypass proxy for local probes`、`docs: separate user and contributor guides`；
- 每个提交保持主题明确，并包含通过该阶段所需的验证；
- 不要在同一提交中混入无关的格式化或重构；
- 拉取请求应说明用户可见变化、实现边界和实际运行过的验证命令。
