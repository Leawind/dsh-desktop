# 贡献指南

感谢你参与 DSH Desktop 的开发。本项目使用 pnpm workspace 管理 Vue 3 前端、UI 组件库和 Tauri 桌面端。

## 开发环境

需要安装：

- Node.js 22.19.0 或更高版本；
- pnpm 11.21.0；
- Rust stable 工具链，并安装 `rustfmt`；
- [Tauri 2 对应平台的系统依赖](https://v2.tauri.app/start/prerequisites/)；
- 可以运行的 `dsh` 命令。

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

应用默认连接 `http://127.0.0.1:3080`。启动前不需要手动运行 `dsh web`：应用会复用该地址上已有的 DSH 服务，或者通过系统中的 `dsh` 命令启动服务。

## 网络与代理

DSH Desktop 自身的 WebView 和 DSH 可达性探测会直接连接目标地址，本地开发不依赖系统代理中的 localhost 忽略规则。

由应用启动的 DSH 子进程仍会继承原有代理环境，因此 DSH 访问模型服务时可以继续使用用户配置的代理。实现方式和平台边界见[网络与代理策略](./docs/design.md#网络与代理策略)。

## 项目命令

所有供本地开发和 CI 使用的命令都定义在根目录的 `package.json` 中。

| 命令                             | 用途                                                         |
| -------------------------------- | ------------------------------------------------------------ |
| `pnpm run dev`                   | 启动完整的 Tauri 开发版本                                    |
| `pnpm run build`                 | 构建当前平台支持的桌面安装包                                 |
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

执行：

```sh
pnpm run build
```

Tauri 会在 Cargo target 目录的 `release/bundle/` 下生成当前操作系统支持的安装文件。未自定义 Cargo target 目录时，默认位置为：

```text
apps/desktop/target/release/bundle/
```

设置了 `CARGO_TARGET_DIR` 或 Cargo 全局 target 目录时，产物会写入对应目录。

- Linux 构建 DEB、RPM 和 AppImage；
- Windows 构建 Windows 安装包；
- macOS 构建 macOS 应用和安装镜像。

不同操作系统的安装包需要在对应平台上构建。

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
