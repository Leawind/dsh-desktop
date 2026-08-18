# DSH 兼容性契约

## 目的

本文记录 DSH Desktop 与 DeepSeek Harness（DSH）之间需要保持兼容的行为。DSH 版本进入内置运行时兼容范围前，必须验证本文列出的契约。具体应用版本允许使用的 DSH 和 Node.js 版本由机器可读的兼容性清单决定。

DSH Desktop 将 DSH Web UI 作为独立网页嵌入，不读取 iframe DOM，也不调用 DSH 内部业务 API。兼容性边界集中在发行包、命令行、进程生命周期、HTTP 服务、iframe、数据目录和插件管理。

## 兼容性状态

DSH 版本相对于一个 DSH Desktop 版本具有以下状态之一：

| 状态         | 含义                                                                                     |
| ------------ | ---------------------------------------------------------------------------------------- |
| `supported`  | 已在目标平台完成兼容性验证，可以作为内置运行时使用                                       |
| `unverified` | 尚未完成当前 Desktop 版本的完整验证，仅供 `system`、`custom` 或 `npx` 来源的用户显式使用 |
| `blocked`    | 已知破坏必要契约，不作为内置运行时安装或启动                                             |

预发布版本必须明确进入 `supported` 集合。版本号位于同一 major、minor 或预发布序列中，不自动代表兼容。

当前内置运行时基线为 DSH `0.1.0-rc.6`、Node.js `24.18.1` 和 pnpm `11.7.0`。Linux x86_64 已完成运行时准备、版本检查、隔离 DSH Home 启动、HTTP 身份探测和随 Host 退出测试。Windows、macOS、其他架构以及插件安装仍需在对应平台完成验证后，才能形成完整的跨平台 `supported` 结论。

## 发行与运行时契约

| ID         | DSH 行为                                               | Desktop 用途                        | 破坏后的影响               | 验证                               |
| ---------- | ------------------------------------------------------ | ----------------------------------- | -------------------------- | ---------------------------------- |
| DIST-001   | DSH 通过可确定版本的 npm 包发布，并声明 `dsh` CLI 入口 | 构建 `bundled` 运行时               | 无法准备内置运行时         | 在目标平台安装确切版本并检查包清单 |
| DIST-002   | 生产依赖可以在支持的平台和架构完成安装                 | 构建包含原生模块的运行时包          | 安装包缺少运行依赖         | 在各目标平台原生安装并执行启动测试 |
| DIST-003   | npm 包元数据提供确切版本与 `dist.integrity`            | 更新前验证下载的 DSH 包             | 未经验证的包可能进入运行时 | 下载 tarball 并比较 SRI SHA-512    |
| DIST-004   | pnpm lockfile 记录安装包的 integrity                   | 记录 staged Runtime 的安装完整性    | 无法追溯实际安装包         | 检查 staged lockfile 的 integrity  |
| NODE-001   | DSH 支持 `bundled` 内置的确切 Node.js 版本             | 运行内置 DSH                        | 进程无法启动或运行异常     | 执行版本、帮助、启动和基本会话测试 |
| PLUGIN-001 | DSH 插件管理使用可用的 pnpm 命令                       | 在 `bundled` 中提供完整插件管理环境 | 插件安装、更新或删除不可用 | 使用私有 PATH 执行插件安装烟雾测试 |

运行时构建保存 DSH、Node.js、pnpm、平台、架构、npm integrity 和文件摘要。构建过程使用锁定的生产依赖，不在用户的全局 npm 环境中安装 DSH。

## 命令行契约

| ID      | DSH 行为                                                           | Desktop 用途                               | 破坏后的影响               | 验证                                 |
| ------- | ------------------------------------------------------------------ | ------------------------------------------ | -------------------------- | ------------------------------------ |
| CLI-001 | `dsh --version` 成功并输出可解析版本                               | 验证 `built-in`、`system` 和 `custom` 来源 | 无法判断运行时身份和兼容性 | 比较退出码与预期版本                 |
| CLI-002 | `dsh web` 启动 Web profile                                         | 创建 Managed Process                       | 无法启动桌面服务           | 在隔离 DSH Home 中启动               |
| CLI-003 | Web profile 接受 `--host 127.0.0.1` 和 `--port <port>`             | 固定端口与端口范围启动尝试                 | 无法在选定端点启动         | 使用固定端口和端口 `0` 分别启动      |
| CLI-004 | 参数错误和端口冲突以非零退出或启动失败呈现                         | 分类启动失败并继续有序尝试                 | 端口范围策略无法可靠继续   | 使用已占用端口执行启动测试           |
| CLI-005 | npm `npx` 可以运行 `@deepseek-ai/dsh` 的 `--version` 与 `web` 命令 | `npx` 来源验证版本并启动 Managed Process   | 无法使用 npm 临时运行 DSH  | 使用无全局 `dsh` 的 Node.js 环境启动 |

DSH Desktop 启动的服务只绑定 `127.0.0.1`。连接类启动尝试和窗口自定义 URL 可以访问用户明确配置的其他地址，但 Host 不通过 Managed Process 公开局域网服务。

## 进程生命周期契约

| ID       | DSH 行为                                                          | Desktop 用途                     | 破坏后的影响              | 验证                             |
| -------- | ----------------------------------------------------------------- | -------------------------------- | ------------------------- | -------------------------------- |
| PROC-001 | `dsh web` 在前台进程中持续运行                                    | 保存进程所有权并监控退出         | Host 无法可靠管理服务状态 | 启动后检查进程和端口持续存活     |
| PROC-002 | 终止 DSH 进程后监听端口能够释放                                   | 停止、重启和更新 Managed Process | 无法在原 URL 重启         | 停止后等待端口释放并原端口重启   |
| PROC-003 | stdout 和 stderr 可以被父进程捕获                                 | 诊断启动和运行错误               | 故障页缺少有效诊断        | 验证正常启动和故障日志采集       |
| PROC-004 | 经 npm `npx` 启动时，DSH 与启动器位于可由 Host 一并终止的进程树中 | 停止、重启和空闲回收 `npx` 来源  | 可能遗留后台 DSH 服务     | 启动后停止服务并检查端口与进程树 |

如果 DSH 将任务转移给独立守护进程、改变信号处理或产生需要单独回收的子进程树，必须重新验证停止和重启流程。

## HTTP 与服务识别契约

| ID       | DSH 行为                                                           | Desktop 用途                        | 破坏后的影响              | 验证                               |
| -------- | ------------------------------------------------------------------ | ----------------------------------- | ------------------------- | ---------------------------------- |
| HTTP-001 | Web 服务在配置的 HTTP URL 返回成功页面                             | Managed Process 就绪检查和窗口加载  | 启动超时或页面不可用      | 轮询根 URL 并检查成功响应          |
| HTTP-002 | 根页面包含可识别的 DSH 身份标记                                    | 区分 DSH 与占用端口的其他 HTTP 服务 | 已有 DSH 被判断为其他服务 | 检查当前版本的身份标记             |
| HTTP-003 | Web 页面资源、HTTP API、事件流和 WebSocket 使用其自身 URL 正常工作 | iframe 中运行完整 DSH Web UI        | 页面加载但会话功能不可用  | 完成加载、设置、会话和流式响应测试 |

当前身份标记是根页面中的：

```html
<title>DeepSeek Harness</title>
```

身份标记属于显式兼容性依赖。DSH 提供稳定的健康与版本端点后，可以将该端点加入契约并用于服务识别。

## iframe 与 WebView 契约

| ID        | DSH 行为                                                                   | Desktop 用途                | 破坏后的影响          | 验证                             |
| --------- | -------------------------------------------------------------------------- | --------------------------- | --------------------- | -------------------------------- |
| EMBED-001 | DSH 响应允许被 DSH Desktop 顶层页面嵌入 iframe                             | 在桌面窗口显示 Web UI       | 浏览器拒绝显示页面    | 检查响应头并在真实窗口加载       |
| EMBED-002 | DSH Web UI 支持现代外部浏览器的 iframe                                     | 在 WebUI 启动的浏览器中使用 | 特定浏览器加载异常    | 三个平台执行基本操作测试         |
| EMBED-003 | iframe 内的同源请求、Cookie、事件流和 WebSocket 不依赖普通浏览器顶层上下文 | 保持客户端与 DSH Host 通信  | iframe 可见但无法工作 | 创建会话并验证流式响应与设置保存 |

需要关注的响应策略包括 `Content-Security-Policy` 的 `frame-ancestors`、`X-Frame-Options`、Cookie 属性和浏览器信任校验。DSH Desktop 不向 iframe 注入 Host bridge 或本地文件权限。

## 数据与工作目录契约

| ID       | DSH 行为                                             | Desktop 用途                                  | 破坏后的影响             | 验证                                 |
| -------- | ---------------------------------------------------- | --------------------------------------------- | ------------------------ | ------------------------------------ |
| DATA-001 | `DSH_HOME` 指定 DSH 配置、凭据、插件和会话数据根目录 | 让所有 Managed Process 使用用户选择的全局目录 | 数据写入错误位置         | 使用指定 DSH Home 启动并检查写入位置 |
| DATA-002 | 未设置 `DSH_HOME` 时使用 `~/.dsh`                    | 支持默认的环境继承模式                        | 默认模式写入错误位置     | 清除环境变量后启动并检查写入位置     |
| DATA-003 | 启动进程的当前目录是默认文件系统位置                 | 为 Managed Process 提供确定的初始目录         | 工作区和工具默认位置异常 | 从指定目录启动并验证初始行为         |
| DATA-004 | 更新后的版本能够读取当前 DSH Home 中的数据           | DSH 热更新和兼容范围内的 Runtime 回滚         | 会话、配置或凭据不可用   | 使用同一 DSH Home 执行升级与回滚测试 |

`runtime/compatibility.json` 的 `rollbackCompatibleRanges` 仅包含已完成 DATA-004 验证的旧版本范围。DSH Desktop 只有在当前 Runtime 位于该范围内时才会执行自动更新和失败自动回滚。

数据格式兼容性与运行时代码兼容性分别验证。验证更新时使用用户选择的同一个 DSH Home；临时验证目录不替代用户目录。

## 视觉兼容性

DSH Desktop 的组件库以兼容 DSH 版本的视觉 token、控件尺寸、间距、动画和交互节奏为基准。视觉变化通常不阻止服务运行，但可能使标题栏、设置层和 iframe 内容失去一致性。每次更新兼容基线时执行并排截图与主要控件交互检查。

## 版本验证流程

一个 DSH 版本进入 `supported` 前执行以下流程：

1. 在 Windows、Linux 和 macOS 的目标架构准备确切运行时；
2. 验证 npm package integrity、运行时清单和第三方许可证；
3. 执行 `dsh --version` 和 `dsh web --help`；
4. 在固定端口、端口 `0` 和已占用端口测试启动行为；
5. 验证根页面、DSH 身份标记和服务停止；
6. 在真实 DSH Desktop iframe 中加载 Web UI；
7. 选择工作区、配置模型、创建会话并验证流式响应；
8. 使用同一个 DSH Home 验证插件管理、重启、升级和兼容范围内的回滚；
9. 更新兼容性清单与本文件引用的上游契约；
10. 保存测试使用的 DSH tag 或 commit，避免以浮动分支作为验证证据。

`system`、`custom` 和 `npx` 来源可以报告 `unverified` 状态并由用户显式继续。`none` 来源和外部端点无法取得版本时，Host 只报告行为探测结果，不声明其版本兼容状态。

## 上游参考

- [DSH 根项目与开发预览状态](https://github.com/deepseek-ai/deepseek-harness)
- [DSH CLI 参数解析](https://github.com/deepseek-ai/deepseek-harness/blob/master/apps/cli/src/args.ts)
- [DSH Web 启动参数](https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/bundle/web-app/src/startup.ts)
- [DSH Web Server](https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/host/webserver/src/index.ts)
- [DSH 插件管理](https://github.com/deepseek-ai/deepseek-harness/blob/master/apps/cli/src/plugin.ts)
- [DSH 第三方依赖说明](https://github.com/deepseek-ai/deepseek-harness/blob/master/THIRD_PARTY_NOTICES.md)
