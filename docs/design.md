# DSH Desktop 设计

## 目标

DSH Desktop 让用户可以像使用普通桌面应用一样使用 DeepSeek Harness（DSH），无需通过命令启动 DSH 或手动配置本地 Web 服务。

应用提供以下能力：

- 支持 Windows、Linux 和 macOS；
- 同时打开多个 DSH 窗口；
- 多个窗口共享同一个本地 DSH 服务；
- 每个窗口可以独立连接指定的 DSH URL；
- 在应用内管理窗口、DSH 服务和 DSH 运行时；
- 独立更新或重启 DSH，无需更新或重启 DSH Desktop；
- 根据兼容性信息选择适合当前应用版本的 DSH 版本。

DSH 的安装、启动方式、运行原理和数据目录等背景信息见[参考](./refer.md)。

## 平台范围

DSH Desktop 面向以下桌面平台发布：

- Windows；
- Linux；
- macOS。

每个平台同时发布两种发行变体：

| 变体      | 内容                                        | 适用场景                                                     |
| --------- | ------------------------------------------- | ------------------------------------------------------------ |
| `bundled` | Tauri 应用、Node.js、DSH、pnpm 和运行时清单 | 安装后直接使用，不要求用户另外安装或下载运行环境             |
| `slim`    | Tauri 应用                                  | 使用系统 `PATH` 中的 DSH、用户指定的 DSH，或者只连接已有服务 |

两种变体提供相同的桌面功能。`bundled` 是面向多数用户的推荐下载，`slim` 提供更小的安装包。发行变体是构建产物的固有能力，不是可以在运行时修改的全局设置。

两个变体使用相同的应用版本号，通过安装包文件名中的 `bundled` 或 `slim` 发行变体字段区分。应用更新保持当前发行变体，不在自动更新过程中切换到另一变体。

## 总体架构

每个系统用户只运行一个 DSH Desktop Host。Host 管理多个桌面窗口、DSH 服务端点和 DSH 运行时。

```text
DSH Desktop Host
├── Window Registry
│   ├── Window A ──→ http://127.0.0.1:3080
│   ├── Window B ──→ http://127.0.0.1:3080
│   └── Window C ──→ https://dsh.example.com
├── Endpoint Registry
│   ├── https://dsh.example.com ──→ External
│   └── http://127.0.0.1:3080 ──→ Managed Process
├── Runtime Registry
│   ├── DSH Runtime 0.1.0-rc.6
│   └── DSH Runtime 0.1.0-rc.7
└── Global Settings and Update Manager
```

核心对象如下：

| 对象             | 职责                                                        |
| ---------------- | ----------------------------------------------------------- |
| Desktop Host     | 保存全局状态，管理窗口、服务、运行时和更新                  |
| App Window       | 承载自定义标题栏、设置界面和一个 DSH iframe                 |
| Service Endpoint | Endpoint Registry 中由规范化 URL 识别的运行时连接目标       |
| Managed Process  | 由 Desktop Host 启动并仍持有进程所有权的本地 `dsh web` 进程 |
| DSH Runtime      | 一个已安装、经过验证且可以独立启动的确切 DSH 版本           |
| DSH Home         | 全局选择的 DSH 配置、凭据、插件和会话数据根目录             |

## 应用实例与窗口

DSH Desktop 使用单进程、多窗口模型。

用户再次启动 DSH Desktop 时，新的启动请求会被转发给已经运行的 Desktop Host。Host 根据请求创建新窗口，随后结束新启动的进程。这样可以同时打开多个独立窗口，同时统一管理本地 DSH 服务。

每个窗口具有独立的：

- 窗口标识；
- 标题、位置和尺寸；
- 连接目标；
- 页面导航和加载状态；
- 窗口级设置。

窗口关闭只会解除该窗口与服务的连接。退出 DSH Desktop 时，Host 负责停止所有由应用管理的 DSH 服务。

## 窗口连接目标

每个窗口直接保存一个规范化后的 DSH URL。

用户可以为特定窗口输入和保存自定义 URL，例如：

```text
http://127.0.0.1:3080
http://localhost:3080
https://dsh.example.com
```

服务端点以 URL 为标识。规范化处理协议和主机名大小写、默认端口和末尾斜杠等不改变含义的形式差异。`localhost`、`127.0.0.1` 和 IPv6 回环地址不作为同一端点合并。

不同 URL 即使最终指向同一个 DSH 进程，也作为不同端点保存。应用不扫描或推断外部服务背后的进程关系。

窗口在创建时获得一个连接目标。服务停止或暂时不可访问时，窗口保留当前 URL 并显示连接状态，不自动切换到其他服务。用户可以在当前窗口设置中显式输入新 URL、选择 Endpoint Registry 中最近连接的服务，或者要求 Host 重新执行窗口启动策略；切换成功后，Host 更新窗口与端点的关联。

## DSH 服务管理

### DSH 原子来源

全局设置保存启动新 DSH 进程时使用的唯一来源：

| 来源       | 含义                                                          |
| ---------- | ------------------------------------------------------------- |
| `none`     | 不启动 DSH 进程，只连接已有服务                               |
| `built-in` | 使用并管理 `bundled` 发行变体附带的 DSH                       |
| `system`   | 从 Desktop Host 的有效 `PATH` 中查找 `dsh`                    |
| `custom`   | 使用用户指定的 DSH 可执行文件或启动脚本                       |
| `npx`      | 使用有效 `PATH` 中的 `npx` 运行 `@deepseek-ai/dsh` 的指定版本 |

`bundled` 支持全部来源；`slim` 不提供 `built-in`。设置界面根据发行变体展示可用来源。读取到当前变体不支持的持久化来源时，Host 保留该设置并要求用户重新选择，不静默切换来源。

首次运行时，`bundled` 默认选择 `built-in`，`slim` 默认选择 `system`。

来源是原子选择，不构成自动回退链。`system`、`custom` 或 `npx` 无法解析、验证或启动时，本次启动尝试失败。已在运行的服务不因全局来源发生变化而被替换；新来源在下一次启动服务或用户显式重启服务时生效。

### 窗口启动尝试

全局设置保存一个有序的窗口启动尝试列表：

尝试按列表顺序执行，第一个成功结果成为窗口连接目标：

| 尝试             | 行为                                                            | 是否使用 DSH 来源 |
| ---------------- | --------------------------------------------------------------- | ----------------- |
| `known-services` | 从 Endpoint Registry 中选择第一个最近成功连接且当前可访问的 DSH | 否                |
| `connect-fixed`  | 连接 `http://<host>:<port>`，不启动进程                         | 否                |
| `start-fixed`    | 在指定回环地址和端口启动新的 DSH；端口被占用时失败              | 是                |
| `start-range`    | 按端口从小到大依次尝试启动新的 DSH                              | 是                |

当前 DSH Web CLI 只用于启动绑定 `127.0.0.1` 的 Managed Process。`connect-fixed` 可以连接用户明确配置的其他 IP 地址；域名、HTTPS 和带路径的目标由窗口自定义 URL 与 Endpoint Registry 承载。

`start-fixed` 不复用端口上已有的 DSH，因为复用行为属于 `known-services` 和 `connect-fixed`。`start-range` 按顺序检查候选端口并尝试启动，端口被占用时继续下一个候选；最终以 DSH 进程的启动和就绪结果为准。

默认策略依次尝试已知服务、连接 `127.0.0.1:3080`，然后在 `127.0.0.1` 的 `3080` 至 `3090` 端口范围内启动服务。

`none` 不影响连接类尝试；启动类尝试遇到 `none` 时以“未配置 DSH 启动来源”失败，并继续执行后续尝试。

### Endpoint Registry 与已知服务

Host 在内存中维护唯一的 Endpoint Registry，不另外保存已知服务列表。注册表以规范化 URL 为键，记录端点状态、进程所有权、关联窗口以及最近一次成功连接的顺序。

任一窗口成功连接端点后，Host 将对应端点标记为已知，并提升到最近成功连接顺序的开头。`known-services` 从该注册表派生候选项，按最近成功连接顺序探测并返回第一个可用服务。一次临时失败不会改变顺序。

Endpoint Registry 只在当前 Host 进程生命周期内存在，不持久化到文件。Host 启动时注册表为空；Host 退出后，最近成功连接顺序随之清空。没有关联窗口的已知外部端点可以保留到当前 Host 退出，以供之后创建的窗口复用。

### 新窗口启动流程

没有显式连接目标的新窗口按以下流程启动：

1. Host 获取窗口服务分配锁，串行化并发的新窗口启动；
2. 依次执行全局窗口启动尝试；
3. 连接类尝试探测目标是否为可访问的 DSH；
4. 启动类尝试按全局 DSH 来源解析并验证启动命令；
5. 第一个成功尝试返回规范化 URL，Host 将其分配给窗口并更新 Endpoint Registry 的最近成功连接顺序；
6. 所有尝试失败时，窗口显示每项尝试及其结构化失败原因。

一次启动流程只解析一次 DSH 来源。来源解析失败时，所有依赖该来源的启动类尝试记录相应错误，列表中后续的连接类尝试仍可继续执行。

失败原因至少区分：Endpoint Registry 中没有已知服务、已知服务均不可用、连接目标不可访问、目标不是 DSH、来源为 `none`、当前变体没有内置运行时、系统 DSH 不存在、自定义路径无效、端口被占用、端口范围耗尽、进程提前退出和启动超时。

### 窗口连接稳定性

窗口启动策略只在窗口没有连接目标，或者用户明确要求重新执行时运行。窗口对应的服务停止或不可访问时：

- 保留原 URL 和窗口与端点的关联；
- 显示服务已停止或连接不可用；
- 可以继续探测并重连同一 URL；
- 不自动执行启动尝试列表；
- 不自动连接 Endpoint Registry 中的其他服务；
- 不自动在其他端口启动服务。

用户可以显式重试当前 URL、重启当前 Managed Process、选择最近连接的服务、输入新 URL，或者重新执行窗口启动策略。手动切换成功后更新窗口关联和 Endpoint Registry 的最近成功连接顺序。旧 Managed Process 是否停止由独立的空闲生命周期策略决定。

### 连接状态

服务状态区分不可访问、正在启动、运行中、正在停止、正在重启、正在更新和失败。

由应用启动的服务完成就绪需要满足以下条件：

1. DSH 子进程仍在运行；
2. DSH 按设定的回环地址和端口监听；
3. HTTP 页面能够访问；
4. 启动过程未超过超时时间。

### 网络与代理策略

DSH Desktop 将桌面壳自身的连接与 DSH 服务进程的连接分开处理。

桌面壳直接连接目标地址，不使用系统代理。这一规则适用于：

- 开发模式下 WebView 加载本地 Vite 服务；
- WebView 中的 DSH iframe；
- Rust Host 对 DSH URL 的存活探测和类型识别；
- 用户为窗口配置的远程 DSH URL。

回环地址是否直连不依赖操作系统代理设置中的 localhost 忽略列表。这样，`127.0.0.1`、`localhost` 和 IPv6 回环地址不会因用户开启系统代理而被发送到代理服务器。

Managed Process 使用另一条环境边界。应用启动时保存原有代理环境，配置桌面 WebView 直连；创建 DSH 子进程时恢复原值。DSH 因此仍可使用用户配置的 `HTTP_PROXY`、`HTTPS_PROXY`、`ALL_PROXY`、`NO_PROXY` 及对应的小写变量访问模型服务和其他外部资源。外部 DSH 进程的代理环境不受应用影响。

各平台的实现如下：

- Linux：在 WebKitGTK 创建首个 WebView 前选择 GIO 的 `environment` proxy resolver，并从桌面进程环境中移除标准代理变量；启动 Managed Process 时恢复这些变量；
- Windows：主窗口和动态创建的窗口都向 WebView2 传入 `--no-proxy-server`；DSH 子进程自然继承原有环境；
- macOS：Rust Host 的 DSH 探测直接连接，WKWebView 当前遵循 macOS 的平台网络策略。macOS 发布验证包含开启系统代理时的本地开发入口、回环 DSH iframe 和自定义远程 URL 测试。

WebKitGTK 和 WebView2 的平台开关分别参考 [WebKitGTK network proxy settings](https://webkitgtk.org/reference/webkit2gtk/2.40.4/method.WebContext.set_network_proxy_settings.html) 和 [WebView2 browser flags](https://learn.microsoft.com/microsoft-edge/webview2/concepts/webview-features-flags)。

### 进程所有权与生命周期

端点的身份与进程所有权分开记录，所有权分为 Host 管理的进程和外部服务。

只有 Desktop Host 启动并仍持有有效进程句柄的 DSH 才是 Managed Process。进程所有权决定应用是否可以停止、重启或更新该 DSH，不参与 URL 去重。

对于已经在运行的外部 DSH，应用不假定能够取得它的工作目录、启动参数、`DSH_HOME`、Runtime 版本或 PID。这些字段保持未知，并且不对该服务提供停止、重启或更新操作。

生命周期规则：

- 关闭一个窗口不会影响仍被其他窗口使用的 Managed Process；
- 没有窗口连接的 Managed Process 进入空闲状态；
- Managed Process 可以在达到可配置的空闲期限后停止；
- 空闲期限默认为 0，表示 Managed Process 没有关联窗口时立即停止；
- 退出应用时停止所有 Managed Process；
- 外部 DSH 不受本地服务生命周期影响；
- 应用只清理具有明确所有权记录的残留进程。

应用记录 Managed Process 的以下信息：

```text
processId
pid
processStartTime
runtimeVersion
dshHome
url
ownerNonce
```

## 界面

### 主窗口

每个主窗口是一个本地 DSH Desktop WebView，由自定义标题栏、内容区和设置层组成。DSH Web UI 使用填满内容区的 iframe 加载。

```text
┌─────────────────────────────────────────────┐
│ ⚙                 DSH             [─][□][×] │
├─────────────────────────────────────────────┤
│                                             │
│               DSH Web UI iframe             │
│                                             │
└─────────────────────────────────────────────┘
```

标题栏取代系统装饰，不再增加额外工具栏。其左侧依次提供设置和刷新按钮，中间区域显示简短标题并用于拖动窗口，窗口控制按钮按平台惯例排列。刷新按钮在当前窗口已连接 DSH 时刷新页面，在窗口启动失败或目标不可访问时重新执行窗口启动策略；启动、停止或重启过程中暂时禁用。macOS 保留原生交通灯按钮，设置和刷新按钮放在同一标题栏区域。

正常使用时，除标题栏外的全部空间都属于 DSH iframe。窗口不常驻显示 URL 栏、服务工具栏或管理侧栏。

### 设置界面

点击标题栏的设置按钮后，设置界面在当前窗口内显示，不创建可以独立拖动的设置窗口。设置层覆盖标题栏下方的内容区，DSH iframe 保持挂载，关闭设置后恢复显示。

设置界面使用与 DSH 设置页一致的居中双栏面板。左侧是分区标签导航，右侧由固定关闭入口和可滚动设置项组成；设置项采用左侧标题与说明、右侧控件的行布局。设置页仅包含“当前窗口”“全局设置”“运行时”和“关于”四项。

设置界面分别呈现当前窗口状态、持久化的全局设置、Desktop Host 的全局运行状态和应用元数据。

设置项修改后自动校验并应用。选择类设置立即反映在界面中，文本和数字输入使用短暂防抖后提交；关闭设置层时会提交仍在等待中的有效更改。Desktop Host 持久化全局设置，并将结果同步到所有已打开窗口。

#### 当前窗口

- 查看和修改当前窗口的 DSH URL。

该页面始终显示当前 URL，使多窗口同时打开设置时仍能明确识别操作对象。

#### 全局设置

全局设置从任意主窗口的设置层访问，修改后由 Desktop Host 同步给所有窗口，包括：

- 界面语言；
- 页面缩放比例；
- DSH 来源；
- DSH Home 来源与自定义目录；
- 有序的窗口启动尝试列表及各项参数；
- 受管服务空闲回收期限，其中 0 表示立即回收。

#### 运行时

- 查看已打开窗口及其连接 URL；
- 聚焦或关闭其他窗口；
- 查看 Endpoint Registry 中的已知 DSH 服务及其 URL、连接状态、进程所有权和已连接窗口数量；
- 停止或重启 Managed Process；
- 查看 Managed Process 的日志和最近错误。

#### 关于

- 查看应用名称、版本和应用标识；
- 查看当前发行变体；
- 查看安装包是否包含内置 DSH 运行时；
- 对于内置运行时，查看 Runtime ID、DSH、Node.js 和 pnpm 版本以及安装状态。

重启、停止或更新共享进程时，界面显示目标 URL 和受影响的窗口数量。

## 前端实现

### 技术栈与编码要求

前端使用 Vue 3、TypeScript 和 Vite 开发，Vue 组件使用 Composition API。单文件组件使用 TypeScript 编写的 `<script setup>`。

项目自有的前端源码、测试、构建脚本和配置脚本使用 TypeScript，不直接编写 JavaScript 业务脚本。工具链支持 TypeScript 配置时使用 `.ts`；工具强制要求 JavaScript 文件时，该文件只保留必要的声明式配置，不承载业务逻辑。

TypeScript 启用严格检查。禁止使用未经说明的 `any`，Tauri IPC、全局状态事件、持久化数据和国际化参数都定义明确类型。构建和 CI 至少执行类型检查、格式检查、静态检查和前端测试。

项目检查、测试和构建命令统一定义在 `package.json` 的 scripts 中。前端脚本使用 `frontend:<name>`，Rust Host 脚本使用 `rust:<name>`；同时涉及前后端且由 CI 直接调用的仓库级脚本不加作用域前缀。本地开发与 CI 调用相同的脚本入口；CI 工作流只负责准备平台工具链、安装系统依赖和调用对应的 pnpm script。

### 前端边界

前端负责界面展示、用户输入、窗口内局部状态和国际化。Rust Host 是窗口列表、全局设置、服务端点、进程所有权和运行时状态的权威数据源。

前端通过集中的类型化 bridge 调用 Tauri 命令并订阅 Host 事件。业务组件不直接调用低层 Tauri API。bridge 统一完成：

- 请求和响应类型定义；
- 结构化错误转换；
- 事件订阅和取消订阅；
- 对窗口和 Host 生命周期的资源清理。

普通组件使用 Vue 响应性和 composable 管理局部状态。只有出现跨多个不相关页面的复杂前端状态时才引入额外状态管理库。

### 组件与仓库目录组织

仓库采用 pnpm workspace 分离桌面 Host、应用前端和通用界面组件：

```text
apps/
├── desktop/
│   ├── src/           Rust Host
│   ├── capabilities/  Tauri 权限配置
│   └── tauri.conf.json
└── frontend/
    ├── src/
    │   ├── bridge/      类型化 Tauri IPC 和事件边界
    │   ├── composables/ Vue 响应式组合逻辑
    │   ├── features/    窗口、服务和设置界面
    │   ├── i18n/        locale 清单和语言资源
    │   └── types/       前端共享类型
    ├── index.html
    └── vite.config.ts
packages/
└── ui/
    └── src/
        ├── components/ 通用 Vue 组件
        └── styles/     设计 token、全局样式和主题
```

`apps/desktop` 是窗口、全局设置、服务进程和持久化状态的实现边界。`apps/frontend` 只包含 DSH Desktop 应用专用的 Vue 界面与前端逻辑。`packages/ui` 提供不依赖应用业务状态的 Vue 组件和样式，并通过 `@dsh-desktop/ui` 包导出。

设置功能位于 `features/settings`。`SettingsOverlay` 只负责设置层外壳、标签导航、页面装配与关闭时提交；“当前窗口”“全局设置”“运行时”“关于”分别由独立页面组件实现。全局设置草稿及自动保存、当前窗口 URL 校验及自动连接各自封装为 composable，页面组件仅处理本页的控件和显示。

页面组件不承担进程控制、持久化或 URL 规范化逻辑。通用交互元素封装为可复用 Vue 组件，包括按钮、输入框、选择器、切换项、对话框、状态标记和错误提示。功能页面使用这些组件，不重复实现基础交互和样式。

### 与 DSH 一致的视觉语言

DSH Desktop 的自有界面与其支持的 DSH Web UI 保持一致的视觉语言，重点包括：

- 背景、表面、边框、主文字、次要文字、强调色和状态色；
- 字体族、字号、字重和行高；
- 间距、圆角、边框、阴影和动画时长；
- 按钮、输入框、对话框、导航和状态反馈的交互形式；
- 浅色与深色主题的对应关系。

这些属性作为语义化 CSS 自定义属性统一定义，组件不直接写入重复的色值和尺寸：

```css
:root {
  --color-background: ...;
  --color-surface: ...;
  --color-text-primary: ...;
  --color-accent: ...;
  --radius-control: ...;
  --space-control-inline: ...;
}
```

设计 token 和通用组件以兼容的 DSH 版本中 `ui-theme` 与 `ui-primitives` 的实现为基准，不从跨域 iframe 的 DOM 或计算样式中动态提取。可直接对应的组件应对齐控件高度、内边距、间距、圆角、字号、行高、交互状态、动画时长、缓动曲线和关键帧；字体族可以根据桌面平台调整。受 Vue、原生表单控件或桌面 WebView 限制时允许存在实现差异，但应保持相同的视觉密度和交互节奏。

组件库维护 100ms、200ms、300ms 三档运动时长和统一缓动曲线，并遵循 DSH 对各组件的实际使用方式；DSH 中没有动画的交互状态不额外添加过渡。按钮、输入框和状态标记等已有直接对应实现的组件优先逐项对齐。选择器触发器沿用输入框的尺寸与状态规则，展开菜单使用自有 DOM 实现，并对齐 DSH Menu 的表面、阴影、间距、选中标记和交互状态。

标题栏、设置层和故障页与 DSH 界面进行并排视觉验证，确保 iframe 边界两侧的色彩、密度和交互反馈连贯。

所有通用组件同时满足键盘操作、可见焦点、语义化标记、足够的颜色对比度和系统减少动画偏好。

## iframe 与权限边界

DSH iframe 是不可信的远程内容边界：

- 只有 DSH Desktop 的本地顶层页面获得完成窗口和服务管理所需的 Tauri capability；
- iframe 中的 DSH 页面不获得 Tauri IPC capability；
- 应用不向 iframe 注入 Tauri API、进程控制函数或本地文件访问能力；
- 顶层页面的 Content Security Policy 只允许 iframe 使用受支持的 HTTP 和 HTTPS 协议，iframe 的 `src` 只由应用在校验窗口 URL 后设置；
- DSH 页面的外部导航按明确的导航策略在当前 iframe 或系统浏览器中打开；
- 修改窗口 URL 时先校验协议和地址，再更新 iframe 和加载策略。

开发构建可以通过窗口 WebView 的开发者工具选中和调试 DSH iframe。

DSH 响应需要允许被嵌入。连接检查会识别 `X-Frame-Options` 或 CSP `frame-ancestors` 造成的嵌入失败，并显示可诊断的错误页和“在系统浏览器中打开”操作。

服务启动、停止、运行时安装和更新均由 Rust Host 执行，不向 DSH 页面暴露相应命令。

Managed Process 仅绑定本机回环地址。DSH Desktop 不通过 Managed Process 提供局域网公开服务。

## DSH 运行时

### 内置运行时

`bundled` 发行变体包含对应平台和架构的 Node.js sidecar、初始 DSH、pnpm、运行时清单及第三方许可证。安装后不依赖系统 Node.js、npm、pnpm 或 `dsh`。Host 使用私有 Node.js 直接运行内置 DSH 的 CLI 入口，并为子进程提供只作用于该进程树的运行时 `PATH`。

安装包附带的 DSH 是内置运行时的初始版本。Host 将运行时安装到应用数据目录，以确切版本保存、验证和切换；多个 DSH 版本可以并存。更新不修改系统全局 npm 环境，也不修改 `system`、`custom` 或 `npx` 来源。

运行时构建由确切的 Node.js、DSH 和 pnpm 版本及生产依赖 lockfile 定义。构建过程校验 Node.js 官方摘要、npm 包 integrity、允许执行的依赖安装脚本、三个入口的实际版本和关键文件摘要。Host 首次使用时将安装包中的种子运行时复制到版本隔离目录并再次校验；运行时定义未变化时，开发和发行构建复用已经准备好的种子目录。

`slim` 发行变体不包含或管理内置运行时。它可以使用 `none`、`system`、`custom` 和 `npx` 来源，并提供与 `bundled` 相同的窗口、连接和服务状态界面。

### 系统、自定义和 npx 运行时

`system` 在每次需要解析启动命令时从有效 `PATH` 查找 `dsh`。Unix 平台优先取得用户登录 Shell 的 PATH，失败时使用 Desktop Host 继承的 PATH；Windows 遵循进程环境和可执行文件扩展规则。Host 记录本次实际解析到的路径和版本，用于运行状态与诊断，不由应用更新或修改该安装。

`custom` 保存用户配置的可执行文件或启动脚本路径。Host 在使用前验证路径、执行 `--version` 并检查兼容性。自定义来源的安装、依赖和更新由用户负责。

`npx` 从有效 `PATH` 解析 `npx`，使用固定包名 `@deepseek-ai/dsh` 启动 DSH。用户可以选择 `latest` 或一个完整的 DSH 版本号；前者允许 npm 在启动时解析新的发布版本，后者提供可复现的版本选择。首次运行或本地 npm 缓存缺少所选包时，npm 下载包及其依赖；下载遵循用户已有的 npm registry、代理和证书配置。Host 不安装、更新或清理 npm 缓存中的包。

`system`、`custom` 和 `npx` 启动的进程在 Host 仍持有进程所有权时也是 Managed Process，因此可以停止和重启。所有 Managed Process 都在独立进程组或等价的进程树范围内启动；停止、重启、空闲回收和 Host 退出时回收整个范围，以覆盖 `npx` 启动器产生的 DSH 子进程。“内置运行时”描述安装与更新所有权，“Managed Process”描述当前进程所有权。

### 目录结构

```text
app-data/
├── runtimes/
│   └── dsh/
│       ├── 0.1.0-rc.6/
│       └── 0.1.0-rc.7/
├── logs/
├── state.json
└── compatibility-cache.json
```

Runtime 可以重新安装或删除，不包含 DSH 的用户配置、凭据、插件和会话数据。用户 workspace 保留在原项目目录中。

### DSH Home

DSH Home 是独立于 Runtime 来源的全局设置。所有由 Desktop Host 启动的 Managed Process 使用同一个选择结果，端点 URL、端口和 Runtime 版本不参与数据目录选择。

全局设置提供两种模式：

- 使用 `$DSH_HOME`：默认模式。Host 不覆盖子进程的 `DSH_HOME`；环境变量未设置时，由 DSH 使用其默认的 `~/.dsh`；
- 自定义目录：Host 将用户填写的绝对路径或 `~/` 路径作为子进程的 `DSH_HOME`。

端点 URL、端口和进程数量不影响 DSH Home。自定义路径由 DSH 按自身规则创建和使用；所选目录不可用时启动直接失败，并保留失败状态和原始日志。修改全局设置不改变正在运行的进程，下一次启动或用户显式重启服务时生效。

## DSH 重启

DSH Desktop 支持在应用和窗口保持运行的情况下替换 DSH 服务进程。

重启流程：

1. 将服务状态切换为 `restarting`；
2. 通知所有关联窗口并显示重连状态；
3. 请求当前 DSH 进程正常退出；
4. 等待 DSH 完成资源释放；
5. 超时后终止残留进程树；
6. 使用相同 Runtime、当前全局 DSH Home、启动配置和端口启动服务；
7. 完成原 URL 的就绪检查；
8. 通知所有关联窗口重新连接。

重启会中断正在执行的请求。执行重启前，应用显示影响范围并要求用户确认。

## DSH 更新

DSH 更新分为准备和切换两个阶段。

### 准备阶段

准备阶段不停止当前服务：

1. 获取兼容性清单；
2. 获取 npm 中可用的 DSH 版本；
3. 根据兼容性策略选择确切版本；
4. 将目标版本安装到 staging 目录；
5. 校验 npm package integrity；
6. 验证版本、命令行和隔离 Web 启动；
7. 将验证通过的目录登记为 Ready Runtime。

### 切换阶段

1. 通知所有关联窗口；
2. 停止旧 DSH 服务；
3. 使用新 Runtime 和当前全局 DSH Home 启动服务；
4. 完成就绪检查；
5. 通知窗口重新加载原 URL；
6. 保留旧 Runtime 供兼容范围内的回滚。

下载和安装新 Runtime 可以在当前服务运行期间完成，服务只在切换阶段短暂停止。

## 更新与数据安全

准备阶段使用一次性的测试目录验证新 Runtime，不将该目录作为用户的 DSH Home。切换阶段只使用用户当前选择的 DSH Home，不复制、替换或迁移其中的数据。

兼容性清单需要明确记录 DSH 数据格式兼容范围。只有确认旧 Runtime 能继续读取当前 DSH Home 时才提供自动回滚；存在破坏性数据迁移风险的版本不进入自动更新范围，并在用户显式更新前提示备份所选目录。

## 版本兼容性

DSH 处于快速迭代阶段，命令行、Web 页面、运行时依赖、嵌入策略和数据格式都可能发生破坏性变化。DSH Desktop 使用远程兼容性清单确定当前应用版本允许使用的 DSH 版本，并以实际验证结果为准。

- DSH major 大于 `0` 时不自动跨 major 更新；
- DSH major 等于 `0` 时不自动跨 minor 更新；
- 预发布版本即使位于同一 minor 版本线，也必须明确加入兼容性清单；
- 内置运行时的安装和自动更新必须位于兼容性清单的允许范围内；
- Runtime 安装始终使用确切版本；
- `system` 和 `custom` 来源的范围外版本标记为未经当前应用版本验证，已知不兼容版本标记为阻止使用；
- `none` 和外部服务无法取得可靠版本时，通过连接与 iframe 行为验证其可用性，不推断版本。

兼容性判断同时考虑 DSH 版本和 `bundled` 发行变体捆绑的 Node.js 版本。DSH Desktop 使用的上游行为、破坏影响和验证方式集中记录在 [DSH 兼容性契约](./dsh-compatibility.md) 中。

## 兼容性清单

兼容性清单由 DSH Desktop 项目维护在仓库中：

```json
{
  "schemaVersion": 1,
  "generatedAt": "2026-08-15T00:00:00Z",
  "apps": {
    "0.1.0": {
      "dsh": {
        "allowedRanges": [">=0.1.0-rc.6, <=0.1.0-rc.9"],
        "recommended": "0.1.0-rc.8",
        "rollbackCompatibleRanges": [">=0.1.0-rc.6, <=0.1.0-rc.8"],
        "blocked": [
          {
            "version": "0.1.0-rc.7",
            "reason": "startup regression"
          }
        ]
      },
      "node": {
        "allowedRanges": ["^22.19.0", ">=24.0.0 <25.0.0"]
      }
    }
  }
}
```

字段含义：

| 字段                       | 含义                                              |
| -------------------------- | ------------------------------------------------- |
| `allowedRanges`            | 当前应用版本已经验证的 DSH 或 Node.js 版本范围    |
| `recommended`              | 默认提供给用户的 DSH 版本                         |
| `rollbackCompatibleRanges` | 更新后的 DSH Home 仍可由旧 Runtime 读取的版本范围 |
| `blocked`                  | 已知不能使用的确切版本及原因                      |

应用内置发布时的兼容性清单，并缓存最近一次验证成功的远程清单。读取优先级如下：

1. 获取并验证远程清单；
2. 使用最近一次有效缓存；
3. 使用应用内置清单。

应用通过 HTTPS 获取仓库中的原始清单，并缓存最近一次成功获取的内容。清单中缺少当前应用版本时，应用继续使用内置兼容范围，不执行超出范围的自动更新。

## 兼容性验证

一个 DSH 版本加入 `allowedRanges` 前，需要在 Windows、Linux 和 macOS 上完成以下验证：

- 安装目标版本；
- `dsh --version` 返回预期版本；
- `dsh web --help` 成功；
- `dsh web --port 0` 成功；
- 能够识别启动 URL；
- Web 首页返回成功响应；
- DSH Web UI 能在系统 WebView 的 iframe 中加载；
- 浏览器与 DSH 的基本通信正常；
- 能够选择工作区并创建会话；
- 模型设置页面可以使用；
- 服务能够正常停止和重启。

兼容性清单以自动化验证结果为依据生成和发布。

## 日志与故障恢复

Desktop Host 保存每个服务的结构化状态和滚动日志：

- DSH stdout 和 stderr；
- 启动、停止和重启事件；
- 端点连接状态变化；
- Runtime 安装和验证结果；
- 更新与回滚结果；
- 进程退出码和最近错误。

服务启动或运行失败时，关联窗口保留原 URL 并显示故障界面。用户可以查看日志、重试启动、切换 Runtime、执行回滚或修改当前窗口的 URL。

## 国际化

### 支持的语言

DSH Desktop 首期支持以下 locale：

| Locale  | 显示名称 |
| ------- | -------- |
| `zh-CN` | 简体中文 |
| `en-US` | English  |

locale 使用 BCP 47 语言标签。全局设置保存用户明确选择的 locale，修改后立即同步到所有已打开窗口，无需重启应用。

首次启动时按以下顺序选择 locale：

1. 已保存的用户选择；
2. 操作系统 locale 的精确匹配；
3. 操作系统语言的默认地区匹配，例如 `zh` 匹配 `zh-CN`；
4. 回退到 `zh-CN`。

### 资源组织

所有由 DSH Desktop 绘制的可见文本、无障碍标签、确认提示和错误消息使用稳定的消息键，不在业务组件中写死展示文本。

语言资源按 locale 分文件保存：

```text
locales/
├── en-US.json
└── zh-CN.json
```

消息键按功能划分命名空间：

```text
window.settings
window.close
service.status.running
service.error.portOccupied
runtime.update.confirm
```

资源格式支持参数插值、复数和选择表达式。日期、时间、数字和文件大小通过当前 locale 的标准国际化 API 格式化，不通过字符串拼接生成完整句子。

`zh-CN` 是缺失消息的回退语言。开发和 CI 检查所有 locale 的消息键集合、参数名和格式是否一致，避免完整发布包中出现混合语言。

### Rust 与前端边界

Rust Host 向前端返回结构化错误代码、插值参数和可选的技术详情。

前端根据 `code` 选择消息键并完成本地化。`technicalDetails`、DSH stdout/stderr 和原始进程错误作为诊断信息保留原文，不作为面向用户的主错误消息。

### iframe 内容边界

国际化覆盖标题栏、设置层、服务管理、更新界面和故障页等 DSH Desktop 自有界面。iframe 中的 DSH Web UI 使用 DSH 自身的语言和 locale 机制。

当 DSH 提供稳定的 locale 设置接口时，DSH Desktop 可以通过独立的适配层将全局 locale 同步给 DSH，不将该集成逻辑写入通用翻译组件。

### 扩展新语言

新增语言时只需：

1. 添加对应 BCP 47 locale 的资源文件；
2. 在 locale 清单中登记标签和本地语言名称；
3. 通过消息键和格式一致性检查；
4. 验证主要窗口在该语言下的布局、截断和无障碍标签。

业务组件、Rust 命令和服务管理逻辑不需要因新增 locale 而修改。

## 实施阶段

### 基础纵切面

完成一条可运行、可测试的最小纵向功能链：

- 搭建 Tauri、Vue 3、TypeScript 和 Vite 工程；
- 建立类型化 Tauri bridge 和基础权限边界；
- 实现单实例、多窗口和自定义标题栏；
- 实现 DSH iframe 和当前窗口的 URL 设置；
- 实现已知服务、固定连接、固定端口启动和端口范围启动策略；
- 实现 `none`、`system` 和 `custom` 来源；
- 实现窗口连接状态和用户显式切换目标；
- 实现窗口内设置层和最小全局设置；
- 实现 `zh-CN` 和 `en-US` 语言资源、locale 切换与回退；
- 建立 TypeScript、国际化资源、Vue 组件和 Rust 核心逻辑的 CI 检查。

该阶段的完成标准是：用户能够按全局启动尝试列表连接或启动系统中的 DSH，在多个窗口中使用 DSH Web UI，并在服务不可用时保持窗口原目标直至用户显式切换。

### 受管运行时与服务管理

- 建立 `bundled` 和 `slim` 两种发行构建；
- 为 `bundled` 提供对应平台的 Node.js、DSH 和 pnpm；
- 安装和验证确切 DSH 版本；
- 实现 `built-in` 来源和版本隔离的内置运行时目录；
- 实现全局 DSH Home 选择，并与 Runtime、端点和端口分离；
- 实现进程所有权、窗口服务分配锁、停止、重启和空闲回收；
- 实现全局窗口、端点、运行时和日志管理；
- 实现多端点与多 Managed Process。

该阶段的完成标准是：用户无需预先安装 Node.js 或 DSH，桌面应用可以完整管理自己启动的 DSH 进程和运行时。

### 安全更新与回滚

- 实现 Runtime staging 和启动验证；
- 实现兼容性清单、缓存和版本选择；
- 实现运行时热切换和兼容范围内的更新失败自动回滚；
- 实现 Runtime 和日志的清理策略；
- 自动化生成并发布兼容性清单。

该阶段的完成标准是：DSH 可以在不更新或重启 DSH Desktop 的情况下更新，始终使用用户选择的 DSH Home，并在数据格式兼容时恢复到已验证的 Runtime。

### 跨平台发行与维护

- 为 Windows、Linux 和 macOS 生成 `bundled` 与 `slim` 安装包；
- 建立 DSH 和 iframe 的跨平台兼容性测试；
- 实现 DSH Desktop 应用更新；
- 完善诊断、故障恢复和日志导出；
- 验证两种语言、浅色与深色主题、键盘操作和主要界面布局。

该阶段的完成标准是：三个目标平台的两种发行变体均能完成全新安装、首次启动、DSH 会话和卸载验证；`bundled` 额外完成内置 DSH 更新验证。

### 后续演进：多子 WebView

当 Tauri 的多子 WebView API 达到稳定状态并通过 Windows、Linux 和 macOS 验证后，可以将 DSH 内容从 iframe 迁移到独立子 WebView。迁移保持现有窗口布局、URL 标识、设置作用域和服务管理语义不变。
