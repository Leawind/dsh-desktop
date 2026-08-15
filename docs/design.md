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

各平台安装包包含 Tauri 应用和运行 DSH 所需的 Node.js 运行时。默认使用由应用管理的 DSH，不要求用户预先安装 Node.js、npm 或 `dsh` 命令。

应用同时支持使用系统中已经安装的 `dsh`，并允许只连接已有 DSH 服务。

## 总体架构

每个系统用户只运行一个 DSH Desktop Host。Host 管理多个桌面窗口、DSH 服务端点和 DSH 运行时。

```text
DSH Desktop Host
├── Window Registry
│   ├── Window A ──→ http://127.0.0.1:3080
│   ├── Window B ──→ http://127.0.0.1:3080
│   └── Window C ──→ https://dsh.example.com
├── Endpoint Registry
│   ├── http://127.0.0.1:3080 ──→ Managed Process
│   └── https://dsh.example.com ──→ External
├── Runtime Registry
│   ├── DSH Runtime 0.1.0-rc.6
│   └── DSH Runtime 0.1.0-rc.7
└── Global Settings and Update Manager
```

核心对象如下：

| 对象 | 职责 |
| --- | --- |
| Desktop Host | 保存全局状态，管理窗口、服务、运行时和更新 |
| App Window | 承载自定义标题栏、设置界面和一个 DSH iframe |
| Service Endpoint | 一个由规范化 URL 识别的 DSH 连接目标 |
| Managed Process | 由 Desktop Host 启动并仍持有进程所有权的本地 `dsh web` 进程 |
| DSH Runtime | 一个已安装、经过验证且可以独立启动的确切 DSH 版本 |
| DSH Home | 一个 Managed Process 使用的配置、凭据、插件和会话数据目录 |

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

每个窗口直接保存一个规范化后的 DSH URL：

```ts
type WindowTarget = {
  url: string;
};
```

用户可以为特定窗口输入和保存自定义 URL，例如：

```text
http://127.0.0.1:3080
http://localhost:3080
https://dsh.example.com
```

服务端点以 URL 为标识。规范化处理协议和主机名大小写、默认端口和末尾斜杠等不改变含义的形式差异。`localhost`、`127.0.0.1` 和 IPv6 回环地址不作为同一端点合并。

不同 URL 即使最终指向同一个 DSH 进程，也作为不同端点保存。应用不扫描或推断外部服务背后的进程关系。

## DSH 服务管理

### 全局默认端口

全局设置保存由应用启动的 DSH 服务的默认端口号。默认端点由回环地址和该端口构造：

```text
defaultDshPort = 3080
defaultDshUrl = http://127.0.0.1:3080
```

修改默认端口只影响之后创建的窗口和之后启动的默认服务，已打开窗口保持当前 URL。

### 新窗口启动流程

在没有显式指定自定义 URL 时，Host 按以下流程创建窗口：

1. 读取全局默认端口并构造默认 URL；
2. 检查该 URL 是否有可访问的 DSH 服务；
3. DSH 可访问时直接创建连接该 URL 的窗口；
4. DSH 不可访问时，获取该 URL 对应的全局启动锁；
5. 获得锁后再次检查 URL；
6. 仍无 DSH 时，在默认端口启动 `dsh web`；
7. 等待服务可访问，然后将窗口连接到默认 URL。

启动锁的键是规范化 URL，用于防止多个窗口重复启动同一端口的 DSH。

检查结果分为：

- 端口未被占用：可以启动 DSH；
- 已有 DSH 可访问：直接连接；
- 端口被其他服务占用：显示端口冲突，由用户修改默认端口或窗口 URL。

应用不自动切换到随机端口。

### 连接状态

```ts
type ServiceStatus =
  | "unreachable"
  | "starting"
  | "running"
  | "stopping"
  | "restarting"
  | "updating"
  | "failed";
```

由应用启动的服务完成就绪需要满足以下条件：

1. DSH 子进程仍在运行；
2. DSH 按设定的回环地址和端口监听；
3. HTTP 页面能够访问；
4. 启动过程未超过超时时间。

### 进程所有权与生命周期

端点的身份与进程所有权分开记录：

```ts
type EndpointOwnership =
  | { type: "managed"; processId: string }
  | { type: "external" };
```

只有 Desktop Host 启动并仍持有有效进程句柄的 DSH 才是 Managed Process。进程所有权决定应用是否可以停止、重启或更新该 DSH，不参与 URL 去重。

对于已经在运行的外部 DSH，应用不假定能够取得它的工作目录、启动参数、`DSH_HOME`、Runtime 版本或 PID。这些字段保持未知，并且不对该服务提供停止、重启或更新操作。

生命周期规则：

- 关闭一个窗口不会影响仍被其他窗口使用的 Managed Process；
- 没有窗口连接的 Managed Process 进入空闲状态；
- Managed Process 可以在达到可配置的空闲期限后停止；
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

标题栏取代系统装饰，不再增加额外工具栏。其左侧提供设置入口，中间区域显示简短标题并用于拖动窗口，窗口控制按钮按平台惯例排列。macOS 保留原生交通灯按钮，设置按钮放在同一标题栏区域。

正常使用时，除标题栏外的全部空间都属于 DSH iframe。窗口不常驻显示 URL 栏、服务工具栏或管理侧栏。

### 设置界面

点击标题栏的设置按钮后，设置界面在当前窗口内显示，不创建可以独立拖动的设置窗口。设置层覆盖标题栏下方的内容区，DSH iframe 保持挂载，关闭设置后恢复显示。

设置界面分为“当前窗口”和“全局设置”两个作用域。

#### 当前窗口

- 查看当前 URL 和连接状态；
- 修改当前窗口的 DSH URL；
- 重新加载页面或重新连接；
- 保存和选择常用 URL；
- 在系统浏览器中打开当前 URL；
- 复制当前 URL；
- 当应用拥有目标进程时查看服务日志；
- 关闭当前窗口。

该页面始终显示当前 URL，使多窗口同时打开设置时仍能明确识别操作对象。

#### 全局设置

全局设置从任意主窗口的设置层访问，修改后由 Desktop Host 同步给所有窗口。其中包含以下页面。

##### 窗口

- 查看已打开窗口及其连接 URL；
- 聚焦或关闭窗口；
- 创建新窗口。

##### 服务

- 查看端点 URL、连接状态和进程所有权；
- 查看已连接窗口数量；
- 启动、停止和重启 Managed Process；
- 更新 Managed Process 使用的 DSH 版本；
- 查看 Managed Process 的日志和最近错误。

##### 运行时

- 查看已安装的 DSH 版本；
- 查看兼容版本和推荐版本；
- 安装、验证或删除 Managed Runtime；
- 查看各运行时的使用情况和磁盘占用；
- 配置系统 `dsh` 可执行文件。

##### 应用设置

- 界面语言；
- DSH 服务默认端口号；
- DSH 更新通道；
- 自动检查和自动下载策略；
- 服务空闲期限；
- 关闭最后一个窗口后的行为；
- 日志保留策略；
- 兼容性清单地址。

重启、停止或更新共享进程时，界面显示目标 URL 和受影响的窗口数量。

## 前端实现

### 技术栈与编码要求

前端使用 Vue 3、TypeScript 和 Vite 开发，Vue 组件使用 Composition API。单文件组件中的脚本使用：

```vue
<script setup lang="ts">
</script>
```

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

### 组件与目录组织

前端按责任组织，页面组件不承担进程控制、持久化或 URL 规范化逻辑：

```text
src/
├── components/       通用界面组件
├── features/         窗口、服务、运行时和更新界面
├── composables/      Vue 响应式组合逻辑
├── bridge/           类型化 Tauri IPC 和事件边界
├── i18n/             locale 清单和语言资源
├── styles/           设计 token、全局样式和主题
└── types/            前端共享类型
```

通用交互元素封装为可复用 Vue 组件，包括按钮、输入框、选择器、切换项、对话框、状态标记和错误提示。功能页面使用这些组件，不重复实现基础交互和样式。

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

设计 token 根据兼容的 DSH 版本维护，不从跨域 iframe 的 DOM 或计算样式中动态提取。标题栏、设置层和故障页与 DSH 界面进行并排视觉验证，确保 iframe 边界两侧的色彩、密度和交互反馈连贯。

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

### 运行时来源

```ts
type RuntimeSource =
  | { type: "managed"; version: string }
  | { type: "system"; executable: string };
```

#### Managed Runtime

Managed Runtime 是默认运行方式：

- 应用捆绑对应平台的 Node.js；
- DSH 安装在应用数据目录；
- 多个 DSH 版本可以并存；
- 安装和更新不修改系统全局 npm 环境；
- 应用负责版本解析、安装、验证和清理。

#### System Runtime

System Runtime 使用用户指定或应用检测到的 `dsh` 可执行文件：

- 保存可执行文件的绝对路径；
- 启动前读取和验证版本；
- 可以由应用启动和停止；
- 不由应用自动更新或修改。

### 目录结构

```text
app-data/
├── runtimes/
│   └── dsh/
│       ├── 0.1.0-rc.6/
│       └── 0.1.0-rc.7/
├── services/
│   └── default/
│       ├── service.json
│       ├── homes/
│       │   ├── generation-1/
│       │   └── generation-2/
│       └── active-home.json
├── logs/
├── state.json
└── compatibility-cache.json
```

DSH Runtime 与 DSH Home 分开保存。Runtime 可以重新安装或删除，DSH Home 保存用户配置、凭据、插件和会话数据。用户 workspace 保留在原项目目录中。

## DSH 重启

DSH Desktop 支持在应用和窗口保持运行的情况下替换 DSH 服务进程。

重启流程：

1. 将服务状态切换为 `restarting`；
2. 通知所有关联窗口并显示重连状态；
3. 请求当前 DSH 进程正常退出；
4. 等待 DSH 完成资源释放；
5. 超时后终止残留进程树；
6. 使用相同 Runtime、DSH Home、启动配置和端口启动服务；
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
3. 从当前 DSH Home 创建新的 generation；
4. 使用新 Runtime 和新 generation 启动服务；
5. 完成就绪检查；
6. 更新 `active-home.json`；
7. 通知窗口重新加载原 URL；
8. 保留旧 Runtime 和旧 generation 供回滚。

下载和安装新 Runtime 可以在当前服务运行期间完成，服务只在切换阶段短暂停止。

## 数据 generation 与回滚

DSH 版本与其写入的数据格式作为一个可回滚单元管理。

```text
升级前：active-home → generation-1

升级时：generation-1 → generation-2

升级成功：active-home → generation-2

升级失败：active-home → generation-1
```

创建 generation 时先停止对源目录的写入。新 Runtime 只使用新 generation，旧 Runtime 和旧 generation 保持配对。

应用根据保留策略清理不再使用的 Runtime 和 generation。复制凭据及其他敏感文件时保持仅当前用户可访问的文件权限。

## 版本兼容性

DSH Desktop 使用远程兼容性清单确定当前应用版本允许使用的 DSH 版本。版本线规则作为附加边界：

- DSH major 大于 `0` 时不自动跨 major 更新；
- DSH major 等于 `0` 时不自动跨 minor 更新；
- 自动更新必须位于兼容性清单的允许范围内；
- Runtime 安装始终使用确切版本；
- 手动选择范围外版本时将其标记为未经当前应用版本验证。

兼容性判断同时考虑 DSH 版本和应用捆绑的 Node.js 版本。

## 兼容性清单

兼容性清单由 DSH Desktop 项目发布，可以托管在 GitHub Pages：

```json
{
  "schemaVersion": 1,
  "generatedAt": "2026-08-15T00:00:00Z",
  "apps": {
    "0.1.0": {
      "dsh": {
        "allowedRanges": [
          ">=0.1.0-rc.6 <=0.1.0-rc.9"
        ],
        "recommended": "0.1.0-rc.8",
        "blocked": [
          {
            "version": "0.1.0-rc.7",
            "reason": "startup regression"
          }
        ]
      },
      "node": {
        "allowedRanges": [
          "^22.19.0",
          ">=24.0.0 <25.0.0"
        ]
      }
    }
  }
}
```

字段含义：

| 字段 | 含义 |
| --- | --- |
| `allowedRanges` | 当前应用版本已经验证的 DSH 或 Node.js 版本范围 |
| `recommended` | 默认提供给用户的 DSH 版本 |
| `blocked` | 已知不能使用的确切版本及原因 |

应用内置发布时的兼容性清单，并缓存最近一次验证成功的远程清单。读取优先级如下：

1. 获取并验证远程清单；
2. 使用最近一次有效缓存；
3. 使用应用内置清单。

远程清单使用 HTTPS 分发并附带离线签名，应用内置公钥完成验证。清单中缺少当前应用版本时，应用继续使用内置兼容范围，不执行超出范围的自动更新。

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

| Locale | 显示名称 |
| --- | --- |
| `zh-CN` | 简体中文 |
| `en-US` | English (United States) |

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

Rust Host 向前端返回结构化错误代码和参数：

```ts
type AppError = {
  code: string;
  args?: Record<string, string | number>;
  technicalDetails?: string;
};
```

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
- 实现默认端口检查，并通过 System Runtime 启动本地 DSH；
- 实现窗口内设置层和最小全局设置；
- 实现 `zh-CN` 和 `en-US` 语言资源、locale 切换与回退；
- 建立 TypeScript、国际化资源、Vue 组件和 Rust 核心逻辑的 CI 检查。

该阶段的完成标准是：用户能够直接启动桌面应用，在默认端口复用或启动系统中的 DSH，并在多个窗口中使用 DSH Web UI。

### 受管运行时与服务管理

- 随应用提供对应平台的 Node.js；
- 从 npm 安装和验证确切 DSH 版本；
- 实现版本隔离的 Managed Runtime 目录；
- 实现 DSH Home 与 Runtime 分离；
- 实现进程所有权、启动锁、停止、重启和空闲回收；
- 实现全局窗口、端点、运行时和日志管理；
- 实现多端点与多 Managed Process。

该阶段的完成标准是：用户无需预先安装 Node.js 或 DSH，桌面应用可以完整管理自己启动的 DSH 进程和运行时。

### 安全更新与回滚

- 实现 Runtime staging 和启动验证；
- 实现兼容性清单、签名、缓存和版本选择；
- 实现 DSH Home generation；
- 实现运行时热切换和更新失败自动回滚；
- 实现 Runtime、generation 和日志的清理策略；
- 自动化生成并发布兼容性清单。

该阶段的完成标准是：DSH 可以在不更新或重启 DSH Desktop 的情况下更新，失败后可以恢复到已验证的 Runtime 和 DSH Home。

### 跨平台发行与维护

- 生成 Windows、Linux 和 macOS 安装包；
- 建立 DSH 和 iframe 的跨平台兼容性测试；
- 实现 DSH Desktop 应用更新；
- 完善诊断、故障恢复和日志导出；
- 验证两种语言、浅色与深色主题、键盘操作和主要界面布局。

该阶段的完成标准是：三个目标平台均能通过安装包完成全新安装、首次启动、DSH 会话、更新和卸载验证。

### 后续演进：多子 WebView

当 Tauri 的多子 WebView API 达到稳定状态并通过 Windows、Linux 和 macOS 验证后，可以将 DSH 内容从 iframe 迁移到独立子 WebView。迁移保持现有窗口布局、URL 标识、设置作用域和服务管理语义不变。
