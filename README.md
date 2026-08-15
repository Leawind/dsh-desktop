# DSH Desktop

> [!IMPORTANT]
>
> 本项目由社区开发，并非 DeepSeek 官方项目，也未获得 DeepSeek 的官方认可或背书。

DSH Desktop 是面向 [DeepSeek Harness（DSH）](https://github.com/deepseek-ai/deepseek-harness) 的跨平台桌面客户端。它会自动启动或连接现有的本地 DSH Web 服务，你可以像使用普通桌面应用一样打开 DSH，而不必手动输入命令。

DSH Desktop 使用 Tauri/Rust + Vue 3 开发。Host 负责窗口、全局设置和 DSH 服务管理，Vue 前端提供桌面界面，并通过 `<iframe>` 显示 DSH Web 界面。

## 功能

- 支持 Windows、Linux 和 macOS
- 启动时自动连接默认端口上的 DSH；服务不存在时通过系统中的 `dsh` 命令启动它
- 在同一个应用进程中打开多个窗口，并尽可能复用同一个本地 DSH 服务
- 为每个窗口单独设置 DSH URL，也可以连接远程 DSH 服务
- 在应用内查看 DSH 服务状态，配置默认端口和 `dsh` 可执行文件
- 提供中英界面

## 安装

当前版本需要系统中存在可用的 `dsh` 命令。可以通过 npm 全局安装 DSH：

```sh
npm install -g @deepseek-ai/dsh
```

DSH 仍处于开发预览阶段，安装和配置方式以其[官方文档](https://github.com/deepseek-ai/deepseek-harness/blob/master/README.zh.md)为准。

DSH Desktop 目前尚未发布稳定安装包。希望试用当前版本时，可以按照[贡献指南](./CONTRIBUTING.md#构建安装包)从源码构建当前平台的安装文件。
