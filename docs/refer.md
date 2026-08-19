# 参考

> 本文档仅包含通过公开途径收集到的客观事实，用于参考。

以下事实截至 2026-08-15。

## 1. 项目身份与状态

- 官方源码仓库为 [deepseek-ai/deepseek-harness](https://github.com/deepseek-ai/deepseek-harness)，默认分支是 `master`。
- 项目由 DeepSeek AI 开发，正式名称为 DeepSeek Harness，命令名为 `dsh`。
- 官方将其定义为 agent harness，而不是单纯的聊天客户端。
- 项目采用 Cordis 驱动的“一切皆插件”架构。
- 当前处于 developer preview，官方明确说明未来会有破坏兼容性的变更。[官方中文 README](https://github.com/deepseek-ai/deepseek-harness/blob/master/README.zh.md)
- 项目采用 MIT License，版权声明为 `Copyright (c) 2026 DeepSeek`。[LICENSE](https://github.com/deepseek-ai/deepseek-harness/blob/master/LICENSE)

## 2. 官方分发与安装方式

### npm

官方面向普通使用者记录的运行方式是：

```sh
npx @deepseek-ai/dsh web
```

前置条件是安装 Node.js。[官方中文 README](https://github.com/deepseek-ai/deepseek-harness/blob/master/README.zh.md)

npm 包信息：

| 字段              | 当前值                  |
| ----------------- | ----------------------- |
| 包名              | `@deepseek-ai/dsh`      |
| 当前版本          | `0.1.0-rc.6`            |
| `latest` dist-tag | `0.1.0-rc.6`            |
| `next` dist-tag   | `0.1.0-rc.6`            |
| rc.6 发布时间     | 2026-08-13 12:35:03 UTC |
| License           | MIT                     |
| npm executable    | `dsh → lib/bin.js`      |
| 发布权限          | public                  |

这些版本信息由 npm 官方 registry 直接查询；包页面是 [@deepseek-ai/dsh](https://www.npmjs.com/package/@deepseek-ai/dsh)。

包暴露了 `dsh` executable，因此通过 npm 全局安装后可以得到持久的 `dsh` 命令：

```sh
npm install -g @deepseek-ai/dsh
```

不过，官方根 README 当前记录的是 `npx @deepseek-ai/dsh web`，没有把全局安装列为主要用法。

### 从源码运行

官方记录的流程为：

```sh
git clone https://github.com/deepseek-ai/deepseek-harness.git
cd deepseek-harness
pnpm install
pnpm run build
pnpm dsh web
```

源码根项目声明：

```json
{
  "packageManager": "pnpm@11.7.0",
  "engines": {
    "node": "^22.19.0 || >=24.0.0"
  }
}
```

即源码声明支持：

- Node.js 22.19.x；
- Node.js 24 及以上。

`@deepseek-ai/dsh@0.1.0-rc.7` 包自身没有包含 `engines`、`os` 或 `cpu` 字段；Node 版本要求来自仓库根项目声明。[根 package.json](https://github.com/deepseek-ai/deepseek-harness/blob/master/package.json)

源码运行需要预先构建 TypeScript host 包和 Web 前端；`pnpm dsh` 本身不会自动构建。npm 安装形式直接运行随包发布的构建产物。[CLI 行为参考](https://github.com/deepseek-ai/deepseek-harness/blob/master/apps/cli/reference/README.zh.md)

## 3. 发布位置

当前正式消费渠道是 npm：

```text
https://www.npmjs.com/package/@deepseek-ai/dsh
```

官方发布工作流具有以下事实：

- `packages/` 和 `apps/` 下的 DSH 包采用统一版本；
- 发布从 `dsh-v*` tag 手动触发；
- 构建并打包 npm tarball；
- 验证 tarball 的全新安装；
- 最终发布到 `https://registry.npmjs.org`；
- vendored framework 和原生 Landlock 包使用独立的版本及发布流程。[release.yml](https://github.com/deepseek-ai/deepseek-harness/blob/master/.github/workflows/release.yml)

当前 GitHub 状态：

- [GitHub Releases](https://github.com/deepseek-ai/deepseek-harness/releases) 页面没有 Release；
- [GitHub Tags](https://github.com/deepseek-ai/deepseek-harness/tags) 页面当前也没有展示可下载版本；
- 没有面向普通用户发布的 `.exe`、`.dmg`、AppImage、`.deb` 或其他 DSH 桌面安装包；
- 主 CLI 当前以 npm/Node.js 包形式发布，不是独立原生可执行文件。

## 4. `dsh` 命令的入口模式

当前 CLI 支持：

| 命令                              | 行为                                   |
| --------------------------------- | -------------------------------------- |
| `dsh web`                         | 启动 Web profile                       |
| `dsh --profile web`               | 与 `dsh web` 等价                      |
| `dsh --profile headless "任务"`   | 运行一次持久化任务，打印最终回答后退出 |
| `dsh --profile <name>`            | 启动指定 profile                       |
| `dsh plugin --profile <name> ...` | 在 profile 目录内调用 pnpm 管理插件    |
| `dsh --dump-config`               | 输出合成后的配置                       |
| `dsh --dump-default-config`       | 输出不含用户层的默认配置               |

`web` 和 `headless` profile 在首次使用时会从内置模板自动初始化。其他名称的 profile 不会自动建立完整应用，需要通过插件命令创建。[CLI README](https://github.com/deepseek-ai/deepseek-harness/blob/master/apps/cli/README.zh.md)

安装或管理第三方插件时，`pnpm` 必须位于 `PATH`；普通的 `dsh web` 启动不以此命令作为显式前置步骤。

## 5. Web 模式的启动事实

普通启动：

```sh
dsh web
```

默认监听地址：

```text
http://127.0.0.1:3080
```

当前 Web profile 支持：

```text
--host <host>
--port <port>
--trusted-host <authority...>
```

当前安装的 `0.1.0-rc.6` 明确支持：

```sh
dsh web --port 0
```

其中端口 `0` 表示由操作系统分配可用端口。

当前 CLI：

- 默认绑定 `127.0.0.1`；
- 有意拒绝 `--host 0.0.0.0`；
- 启动后向标准输出打印 URL；
- 没有 `--open` 或 `--no-open` 参数；
- 官方文档描述的是“打印访问地址”，未记录自动启动浏览器的行为。[Web bundle 配置](https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/bundle/web-app/cordis.patch.yml)

本机隔离实测：

```text
dsh web: http://127.0.0.1:37053
```

访问 `/` 返回：

```text
HTTP/1.1 200 OK
content-type: text/html; charset=utf-8
```

该实测使用临时 `DSH_HOME`，测试完成后已删除临时数据。

## 6. Web 模式运行原理

`dsh web` 在一个长期运行的 Node.js 进程中组合并启动插件树。

Web profile 的主要组成包括：

- `dsh-base`：模型、会话、工具、权限、沙箱、设置、凭据等基础能力；
- `dsh-web-app`：Web 服务、Web 前端、API gateway 和浏览器端插件；
- HTTP server：默认监听 `127.0.0.1:3080`；
- 静态前端服务；
- `/api` transport；
- 浏览器端通过 fetch/SSE 与 host 端 gateway 通信；
- JSON storage；
- workspace、会话投影、设置、模型选择和审批界面。

这不是一个只提供静态页面的命令：Agent 运行时、模型适配器、工具执行和 Web 服务都由该 DSH 进程承载。[Web profile 配置](https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/bundle/web-app/cordis.patch.yml)

Harness 的会话日志是持久化事实来源。UI、恢复、fork、transcript 和模型上下文都从会话事件流派生。[架构文档](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/architecture.md)

## 7. 工作目录与工作区

- 启动 `dsh` 时的当前目录是默认 workspace root。
- Web UI 第一次打开时不会自动选中工作区。
- 用户需要在 Web UI 中添加并选择工作区。
- 未选择工作区前，会话输入框不可用。
- Agent 可以读取和修改工作区文件、运行命令、委派任务和维护计划。
- 需要审批的操作会通过 Web UI 请求用户确认。[Web UI 指南](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/user/guide/index.zh.md)

所有运行模式都会在默认 workspace 中查找适用的 `AGENTS.md` 或 `CLAUDE.md`。

## 8. 用户数据与配置位置

DSH 用户数据根目录的优先级为：

1. 程序显式指定路径；
2. `$DSH_HOME`；
3. 默认 `~/.dsh`。

官方说明 Harness 用户数据集中在这个根目录下。[dsh-home-paths](https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/util/home-paths/README.md)

已知路径包括：

```text
$DSH_HOME/
├── profiles/
│   ├── web/
│   └── headless/
├── storages/
├── .credentials.yaml
├── settings.yaml
├── cordis.patch.yml
└── .env
```

Web profile 第一次启动会初始化 profile，并在后续启动时维护 profile 的模块解析符号链接。

配置层次依次为：

1. profile 中声明的 bundle；
2. profile 的 `cordis.patch.yml`；
3. `$DSH_HOME/cordis.patch.yml`；
4. 命令行中的 `--patch`。

profile 和 home 两个 `cordis.patch.yml` 会在运行期间被监视，并在有效修改后重新应用。

## 9. 模型和凭据

Web UI 中可以通过“设置 → 模型”完成配置，不需要通过命令行设置服务。

支持：

- DeepSeek；
- Anthropic；
- OpenAI；
- Bedrock；
- Vertex；
- Azure；
- Codex；
- 自定义 OpenAI 兼容端点；
- 其他已安装 provider catalog 提供的模型。

模型或 provider 修改在下一次请求时生效，不需要重启 DSH。

DeepSeek API Key：

- 可以在 Web UI 输入；
- 保存于 `$DSH_HOME/.credentials.yaml`；
- settings 中只保存凭据引用；
- 前端保存后只能收到脱敏描述，不会重新收到明文密钥。[模型配置指南](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/user/guide/providers.zh.md)

凭据解析顺序为：

1. 继承的进程环境；
2. `$DSH_HOME/.credentials.yaml`；
3. 启动目录下的 `.env`；
4. `$DSH_HOME/.env`。

DSH 不捆绑本地大模型。运行模型请求需要配置外部 provider、API Key 或自定义 endpoint。

## 10. 进程退出行为

官方定义：

- 配置、参数或插件启动失败时以非零状态退出；
- 收到第一次 `SIGTERM` 时执行正常关闭，并以 `0` 退出；
- 收到第一次 `SIGINT` 时执行正常关闭，并报告 `130`；
- 插件树最多有 5 秒执行 dispose；
- 第二次收到信号时立即强制退出。

本机对 `0.1.0-rc.6` 的实测结果：

```text
Ctrl+C → exit code 130
```

[CLI 关闭行为参考](https://github.com/deepseek-ai/deepseek-harness/blob/master/apps/cli/reference/README.zh.md)

## 11. 平台支持的现有证据

官方当前没有提供一张明确的“正式支持平台矩阵”。

可以确认的客观事实是：

- npm CLI 包没有 `os` 或 `cpu` 安装限制；
- 主要 CI 在 Linux/Node 24 上运行；
- Node 22.19 和 Node 26 有兼容性测试；
- 仓库有真实 Windows runner，并运行完整的 native Windows gate；
- 仓库还有通过 Wine 执行的 Windows blocking gate；
- macOS 有真实的 Seatbelt sandbox E2E；
- 完整 macOS serial CI job 存在，但当前配置为 `if: false`；
- Linux sandbox E2E 覆盖：
  - Ubuntu x64 + bubblewrap；
  - Ubuntu x64 + Landlock；
  - Ubuntu ARM64 + Landlock；
- macOS sandbox 使用 Seatbelt；
- Windows sandbox 使用 ACL restricted token，并被官方标为部分强制执行。[CI](https://github.com/deepseek-ai/deepseek-harness/blob/master/.github/workflows/ci.yml) [Sandbox CI](https://github.com/deepseek-ai/deepseek-harness/blob/master/.github/workflows/sandbox.yml) [沙箱实现说明](https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/sandbox/sandbox-local/README.zh.md)

因此，目前有明确代码和 CI 证据覆盖 Linux、Windows、macOS，但官方尚未声明具体最低操作系统版本、发行版范围或完整正式支持等级。

## 12. 当前本机环境

本机实际状态：

```text
OS:       Linux 6.8.0-136-generic x86_64
Node.js:  v24.19.0
npm:      11.17.0
DSH:      0.1.0-rc.6
```

`dsh` 当前路径：

```text
/home/leawind/.nvm/versions/node/v24.19.0/bin/dsh
```

解析后的实际入口：

```text
/home/leawind/.nvm/versions/node/v24.19.0/lib/node_modules/@deepseek-ai/dsh/lib/bin.js
```

即本机的 `dsh` 安装在 NVM 管理的 Node.js 全局 npm 目录中。

当前 `dsh-desktop` 仓库：

```text
branch: main
HEAD: ca721f5 init
tracked files: .gitignore
worktree: clean
```

## 13. 官方资料暂未确认的事项

目前没有找到官方明确承诺的：

- 完整操作系统支持矩阵；
- 最低 Windows 版本；
- 最低 macOS 版本；
- Linux 发行版与最低 glibc 要求；
- 面向普通用户的独立 DSH 可执行文件；
- 桌面安装包；
- 自动更新接口；
- 稳定的机器可读服务就绪协议；
- 启动 URL 输出格式的长期兼容承诺；
- Web API 的长期稳定公共协议；
- DSH 与第三方桌面宿主之间的官方集成规范。

这些属于当前官方资料中未确认的范围，而不是已经确认不支持。
