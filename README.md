<div align="center">

# DSH Desktop

DSH Desktop 是 [DeepSeek Harness（DSH）](https://github.com/deepseek-ai/deepseek-harness) 的跨平台（Windows / Linux / macOS）桌面客户端。

它会自动启动或连接现有的本地 DSH Web 服务，你可以像使用普通桌面应用一样打开 DSH，而不必手动输入命令。

</div>

## 安装

每个平台提供两种可直接运行的可执行文件：

- `slim`：体积极小（< 10MB），适合自行安装或启动 DSH 的用户。
- `bundled`：包含独立的 Node.js、DSH 和 pnpm，直接运行即可启动 DSH；

使用 `slim` 时，可以通过 npm 全局安装 DSH：

```sh
npm install -g @deepseek-ai/dsh
```

也可以在应用的全局设置中选择“通过 npx 运行 DSH”，填写 `latest` 或一个确切版本。该方式要求系统安装 Node.js；首次启动会由 npm 下载所选 DSH 版本。

Deepseek Harness 仍处于开发预览阶段，安装和配置方式以其[官方文档](https://github.com/deepseek-ai/deepseek-harness/blob/master/README.zh.md)为准。

> [!IMPORTANT]
>
> 本项目由社区开发，并非 DeepSeek 官方项目，也未获得 DeepSeek 的官方认可或背书。
