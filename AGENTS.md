# AI Agent 指南

## 项目阶段

- 本项目尚未正式发布。
- 不为开发阶段产生的旧配置、旧数据结构、旧命令或旧接口保留向后兼容逻辑。
- 数据模型发生变化时直接更新当前实现、测试和文档，不添加迁移器、兼容分支、旧字段别名或弃用层。
- 如果本地存在无法被当前结构读取的开发配置，允许应用回退到当前默认设置。

## 开发环境

- 使用 Node.js 22.19.0 或更高版本、pnpm 11.21.0、Rust stable 和 `rustfmt`。
- WebUI 和托盘构建需要 `make`、C/C++ 编译器和目标平台所需的系统库；Linux CI 的基础依赖为 `build-essential`、`curl`、`wget`、`file`、`libgtk-3-dev` 和 `libappindicator3-dev`。
- 运行 WebUI 窗口需要系统中存在受支持的浏览器；运行 `slim` 变体还需要可用的 `dsh` 命令。

## 设计与文档

- `docs/design.md` 描述宏观设计和当前决策，不堆放没有必要的代码类型定义或实现细节。
- 文档只正面描述当前采用的设计，除非用户明确要求，不记录已经放弃的方案及其淘汰原因。
- `docs/dsh-compatibility.md` 记录 DSH Desktop 实际依赖的上游契约；实现新增依赖时同步更新该文档。

## 架构约束

- 仓库中的桌面 Host、Vue 前端和 UI 组件库保持目录分离：
  - `apps/desktop`：WebUI/Rust Host；
  - `apps/frontend`：Vue 3 应用；
  - `packages/ui`：与 DSH 视觉语言对齐的可复用组件。
- 前端使用 Vue 3 和 TypeScript，不直接编写 JavaScript 源文件。
- UI 不只对齐 DSH 的颜色，还应尽量对齐控件尺寸、间距、圆角、动画和交互节奏；允许因平台 WebView 或实现成本存在有依据的差异；不必确保符合最新 DSH 的实现，仅当用户明确要求时才更新。
- 用户界面支持 `zh-CN` 和 `en-US`，缺失语言或翻译时回退到 `zh-CN`。新增文案必须同时更新两种语言，并保持未来添加语言的结构清晰。

## DSH 服务模型

- URL 是端点身份。不要尝试合并 `localhost`、`127.0.0.1` 或实际指向同一进程的不同 URL。
- 外部服务可能无法报告工作目录、启动配置、PID 或 DSH Home；未知信息保持未知。
- 多窗口启动服务时必须串行化关键分配过程，尽量避免重复启动 DSH。
- 所有受管 DSH 服务使用全局选择的 DSH Home。默认继承 `$DSH_HOME`，也允许用户指定自定义目录。
- 不要根据 URL、端口或进程静默生成新的 DSH Home；所选目录不可用时直接报告启动失败，不回退到其他目录。

## 命令与验证

- CI 和开发工作流需要的命令统一定义在根 `package.json`。
- 前端脚本使用 `frontend:<name>`，Rust 脚本使用 `rust:<name>`；CI 直接执行的聚合脚本不加前缀，例如 `format`、`format-check`、`check`。
- 完成实现后至少运行 `pnpm check`。涉及窗口、WebView、进程、代理或服务生命周期时，还要进行与风险相称的真实运行验证。
- `pnpm dev` 同时启动 Vite 和 Rust Host；浏览器通过 Vite 的 `/api` 代理访问 Host，前端修改应通过热更新生效。再次运行时复用已有 Vite 与 Host，由 Host 创建新的浏览器窗口；最后一个窗口关闭后，Host 在仍有受管 DSH 服务时保留托盘并继续运行，全部受管服务被回收后 Host 和该命令启动的 Vite 一起退出。
- `pnpm build:bundled` 和 `pnpm build:slim` 生成当前平台可独立移动运行的单文件可执行程序，产物位于 Cargo target 目录下的 `release/artifacts/`。
- `runtime/versions.json` 固定内置运行时版本，`runtime/package-lock.json` 固定生产依赖闭包；修改内置运行时前先运行 `pnpm run runtime:prepare`。

## 前端约定

- 用户可见文本同时提供 `zh-CN` 和 `en-US`；通用 UI 能力放在 `packages/ui`，应用业务逻辑、HTTP bridge 和页面放在 `apps/frontend`。

## Git 工作流

- 提交信息使用英语和 Conventional Commits 风格。
- 复杂任务应在形成可构建、职责完整的阶段时及时提交，不必等到所有规划功能全部完成。
- 除非用户明确要求，否则不要推送提交。

### 发布版本

- 发布版本以 `apps/desktop/Cargo.toml` 的 `[package].version` 为准，发布标签必须为完全匹配的 `v<version>`。
- Windows MSI 仅接受纯数字且不大于 `65535` 的预发布标识；需要构建 MSI 时，不要使用 `-rc.2` 这类含文本或多段的预发布版本，改用正式版本或类似 `-2` 的单个数字标识。
- 先运行 `pnpm run release:version -- <version>`，它会更新 Cargo 包版本、`Cargo.lock`，并在 `CHANGELOG.md` 顶部创建对应版本小节。
- 在提交前填写该版本的可见更新日志；发布工作流会将其作为 GitHub Release notes，空小节或仅有占位注释均不允许。
- 提交版本与更新日志文件后，运行 `pnpm check` 和 `pnpm run release:prepare -- v<version> <release-notes-output>` 验证版本、标签和发行说明。
- 验证通过后，创建指向该提交的附注标签 `v<version>`。仅在用户明确要求时推送提交与标签。
