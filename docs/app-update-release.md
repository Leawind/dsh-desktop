# 应用更新发布

DSH Desktop 使用 Tauri 更新器发布应用自身的更新。更新机制按 `bundled` 和 `slim` 分开，客户端不会在更新时切换发行变体。

## 签名密钥

更新公钥位于 `apps/desktop/tauri.conf.json`，并被打包进应用。与它匹配的私钥不能提交到仓库，也不能写入日志或 Release 资产。

将私钥内容保存为 GitHub Actions Secret `TAURI_SIGNING_PRIVATE_KEY`。如私钥设置了密码，同时配置 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。发布工作流会将两个 Secret 传递给 Tauri 构建；缺少或不匹配时，Tauri 不会生成有效的更新签名，发布应失败。

密钥仅生成一次并长期保留。丢失该私钥后，已发布应用无法信任新更新；更换密钥需要通过新的应用版本完成明确的密钥轮换。

## 发布内容

发布标签触发构建后，Tauri 为各平台生成更新安装包和 `.sig` 签名。发布工作流收集所有平台的产物并生成：

- `latest-bundled.json`；
- `latest-slim.json`。

每份清单只包含对应变体的 Linux AppImage、Windows NSIS 安装程序和 macOS 应用归档。清单中的下载 URL 固定到本次 `v<version>` Release，签名内容直接写入清单。

客户端通过 GitHub Release 的 `latest/download/latest-<variant>.json` 获取清单，因此仅跟随最新稳定 Release。预发布包不会覆盖稳定更新通道。

## 发布前检查

按照仓库的常规发布流程准备版本、更新日志和标签。除 `pnpm check` 与 `pnpm run release:prepare` 外，确认 GitHub Actions 已配置签名 Secret。发布完成后，在每个目标平台的已安装旧版本中执行“关于 → 检查更新 → 安装并重启”，验证应用重启后的版本号和当前发行变体。
