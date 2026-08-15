# DSH Desktop

DSH Desktop 是基于 Tauri 和 Vue 3 的 DeepSeek Harness 桌面客户端。应用可以复用默认端口上已经运行的 DSH，也可以通过系统中安装的 `dsh` 命令启动本地服务。

当前实现包括：

- 单个 Host 进程管理多个应用窗口；
- 自定义标题栏和常驻的 DSH iframe；
- 当前窗口 URL 设置和全局默认端口设置；
- 系统 `dsh` 探测、启动、进程所有权和退出清理；
- `zh-CN` 和 `en-US` 界面语言；
- Vue 3 内部 UI 组件层及与 DSH 对齐的设计 token。

详细设计见[设计文档](./docs/design.md)，DSH 的客观背景信息见[参考资料](./docs/refer.md)。

## 仓库结构

```text
apps/
├── desktop/   Tauri 配置、Rust Host 与桌面应用构建入口
└── frontend/  Vue 3 应用界面与 Vite 构建入口
packages/
└── ui/        可复用的 Vue 组件与设计 token
```

根目录的 `package.json` 和 `pnpm-workspace.yaml` 负责工作区编排，并为本地开发与 CI 提供统一命令。应用专用界面保留在 `apps/frontend`，不含 DSH Desktop 业务逻辑的通用界面能力放在 `packages/ui`。

## 开发环境

需要安装：

- Node.js 22.19.0 或更高版本；
- pnpm 11.21.0；
- Rust 工具链；
- Tauri 对应平台的系统依赖；
- 可以运行的 `dsh` 命令。

Debian 或 Ubuntu 上可以安装 Tauri 的 Linux 开发依赖：

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

安装项目依赖并启动开发版本：

```sh
pnpm install
pnpm run dev
```

应用默认连接 `http://127.0.0.1:3080`。也可以在全局设置中指定 `dsh` 可执行文件的绝对路径。

DSH Desktop 自身的 WebView 和 DSH 可达性探测直接连接目标地址，不要求在系统代理中额外配置 localhost 忽略规则。应用启动的 DSH 子进程仍继承原有代理环境，因此 DSH 访问模型服务时可以继续使用用户配置的代理。具体的平台实现和边界见[网络与代理策略](./docs/design.md#网络与代理策略)。

## 检查

运行与 CI 相同的全部格式检查、静态检查、测试、类型检查和前端生产构建：

```sh
pnpm run check
```

也可以分别执行具体检查：

```sh
pnpm run format-check
pnpm run frontend:lint
pnpm run frontend:test
pnpm run rust:test
```

构建不含平台安装包的桌面可执行文件：

```sh
pnpm run build
```
