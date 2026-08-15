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
pnpm tauri dev
```

应用默认连接 `http://127.0.0.1:3080`。也可以在全局设置中指定 `dsh` 可执行文件的绝对路径。

## 检查

运行前端格式、静态检查、测试、类型检查和生产构建：

```sh
pnpm check
```

运行 Rust 格式和测试：

```sh
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
```
