# DSH Desktop

> [!IMPORTANT]
>
> 本项目由社区开发，并非 DeepSeek 官方项目，也未获得 DeepSeek 的官方认可或背书。

DSH Desktop 是面向 [DeepSeek Harness（DSH）](https://github.com/deepseek-ai/deepseek-harness) 的跨平台桌面客户端。它会自动启动或连接现有的本地 DSH Web 服务，你可以像使用普通桌面应用一样打开 DSH，而不必手动输入命令。

DSH Desktop 使用 Rust + WebUI + Vue 3 开发。Host 负责窗口、全局设置和 DSH 服务管理；WebUI 使用已安装浏览器承载 Vue 前端，并通过 `<iframe>` 显示 DSH Web 界面。

## 功能

- 支持 Windows、Linux 和 macOS
- `bundled` 安装包内置 Node.js、DSH 和 pnpm，安装后即可启动 DSH
- `bundled` 可在不更新桌面应用的情况下检查并安全更新已验证的内置 DSH 运行时
- 可以检查并安装与当前 `bundled` 或 `slim` 发行变体匹配的已签名 DSH Desktop 更新
- `slim` 安装包体积更小，可使用系统中的 `dsh`、`npx`、自定义命令路径或已有服务
- 启动时按可配置的尝试顺序连接已有服务或启动新的 DSH
- 在同一个应用进程中打开多个窗口，并尽可能复用同一个本地 DSH 服务
- 为每个窗口单独设置 DSH URL，也可以连接远程 DSH 服务
- 在应用内查看 DSH 服务、最近日志和运行时状态，停止或原地址重启受管服务
- 全局选择继承 `$DSH_HOME` 或使用自定义 DSH 用户数据目录
- 提供中英界面

## 安装

每个平台提供两种安装包：

- `bundled`：推荐给大多数用户，包含独立的 Node.js、DSH 和 pnpm；
- `slim`：安装包更小，适合已经安装 DSH、Node.js 或只连接已有服务的用户。

使用 `slim` 时，可以通过 npm 全局安装 DSH：

```sh
npm install -g @deepseek-ai/dsh
```

也可以在应用的全局设置中选择“通过 npx 运行 DSH”，填写 `latest` 或一个确切版本。该方式要求系统安装 Node.js；首次启动会由 npm 下载所选 DSH 版本。

DSH 仍处于开发预览阶段，安装和配置方式以其[官方文档](https://github.com/deepseek-ai/deepseek-harness/blob/master/README.zh.md)为准。

DSH Desktop 目前尚未发布稳定安装包。希望试用当前版本时，可以按照[贡献指南](./CONTRIBUTING.md#构建安装包)从源码构建当前平台的 `bundled` 或 `slim` 安装文件。

维护者可以查看[应用更新发布说明](./docs/app-update-release.md)，了解签名密钥和 Release 更新清单的配置方式。
