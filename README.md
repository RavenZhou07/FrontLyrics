# LyricGlass

Windows 桌面歌词悬浮窗。**Tauri 2 + React / TypeScript + Rust**；不播放、不截取音频，不需要 Spotify OAuth。音乐来自播放器，歌词来自独立歌词服务或本地文件。

## 运行

环境：Windows 10 1809+ / Windows 11、Node.js 22.12+（本次开发使用 Node 24）、Rust stable 的 `x86_64-pc-windows-msvc` 工具链、Visual Studio 2022 Build Tools 的 **Desktop development with C++** 工作负载及 Windows SDK、Microsoft Edge WebView2 Runtime。见 [Tauri Windows 环境要求](https://v2.tauri.app/start/prerequisites/#windows)。首次安装依赖和打包需要网络。

```powershell
npm ci
npm run dev
```

`npm run dev` 同时启动 Vite 和原生应用。打开 Spotify Desktop 或网易云音乐并播放，应用会枚举 Windows 媒体会话并查找歌词。播放器必须向 Windows 发布 SMTC；应用无法补足播放器未公开的数据。网易云 3.1.41 的开关位于 **设置 → 系统 → 开启 SMTC**。本机该版本仅发布标题/艺人/状态，未提供时间轴，因此本机网易云自动同步验收仍未通过；见下文与验收记录。不要将 `npm run dev:web` 的浏览器预览视为原生应用；它明确不提供 SMTC。

```powershell
npm test                  # 前端与 Rust 单元测试
npm run check             # TypeScript + Rust Clippy（警告视为失败）
npm run build:web         # 前端生产构建
npm run probe:smtc        # 只读枚举真实 SMTC，输出 JSON，不打印封面
npm run probe:lyrics      # 请求真实 LRCLIB，输出类型/行数/置信度，不打印歌词
npm run build             # Release 原生程序 + 当前用户 NSIS 安装包
npm run format            # 格式化 TypeScript/CSS/JSON/Rust
```

安装包：`src-tauri/target/release/bundle/nsis/LyricGlass_0.1.0_x64-setup.exe`。未配置代码签名。开发命令首次运行会自动由 Cargo 下载、编译 Rust 依赖。依赖版本由 `package-lock.json` 与 `src-tauri/Cargo.lock` 固定。

## 使用

- **编辑**：拖动歌词或上方工具条移动；右下角手柄调整大小。**锁定**：禁止拖动缩放，双击打开设置。**穿透**：鼠标穿过整个窗口。托盘始终可以找回窗口、切换模式、显示/隐藏、打开设置或退出。
- 默认快捷键：`Ctrl+Alt+Shift+L` 循环三种模式；`Ctrl+Alt+Shift+←` 歌词提前 100 ms；`Ctrl+Alt+Shift+→` 延后 100 ms。可改绑，注册失败保留原绑定。额外的 Shift 避免与网易云默认热键冲突。快捷键调整的是本曲临时偏移；换曲归零。全局偏移单独保存。**正偏移表示歌词提前**。
- 设置的预览默认显示原创示例文案；切到“正在播放”检查当前歌曲。外观改动立即广播到真正的悬浮窗。主题、中文字体、颜色、渐变、阴影、描边、透明度、单/双/三行、留白全部可编辑。
- 导入 UTF-8 `.lrc` / `.yrc` / `.txt` 为当前曲目的本地覆盖。带相同时间戳的第二条不同文本会作为翻译；多个相同文本去重。可以移除覆盖恢复网络匹配。纯文本完整滚动显示，**不伪造同步**。
- 主题是人类可读 JSON，示例在 `themes/`。可以命名保存、导入与通过原生保存对话框导出。颜色要求 `#RRGGBB`，未知/不合法的数值不会直接应用到界面。
- 多屏位置和窗口尺寸保存在物理像素坐标；移除显示器后会将完全不可见的悬浮窗居中。托盘“找回悬浮窗”可恢复默认大小和编辑模式。

## 实现结构

```text
Windows 播放器
  → Windows.Media.Control / GSMTC（专用 MTA 线程）
  → 统一 Playback + 会话仲裁 + 单调时钟
  → LyricsProvider → 保守匹配 → 本地缓存 → 统一 Lyrics
  → Tauri 状态事件 → 独立 React 悬浮窗
```

| 模块 | 职责 |
| --- | --- |
| `src-tauri/src/platform/windows.rs` | 枚举全部会话、WinRT 事件订阅、封面读取、系统字体、时间轴快照；异步读取超时与重连 |
| `src-tauri/src/playback.rs` | 可独立测试的会话仲裁、单调时钟外推、权威时间轴校正策略 |
| `src-tauri/src/model.rs` | 与播放器和歌词服务无关的播放/歌词模型 |
| `src-tauri/src/lyrics/providers.rs` | `LyricsProvider`、真实 LRCLIB、本地覆盖、隔离的网易云歌词数据转换边界 |
| `src-tauri/src/lyrics/parser.rs` | 普通/多时间戳/增强 LRC、YRC、翻译、偏移、异常输入 |
| `src-tauri/src/lyrics/matching.rs` | Unicode 规范化、稳定 SHA-256 曲目键、标题/艺人/时长匹配及置信度 |
| `src-tauri/src/lyrics/cache.rs`、`resolver.rs` | 30 天正缓存、10 分钟负缓存、质量优先与故障回退 |
| `src-tauri/src/app.rs`、`settings.rs` | IPC、代际保护、托盘、全局快捷键、设置校验/备份、窗口状态 |
| `src/LyricsView.tsx`、`timeline.ts` | 与歌词来源无关的渲染、逐词进度、暂停/间奏处理、封面取色 |
| `src/Overlay.tsx` | 独立透明、无边框、置顶窗口与编辑手柄 |
| `src/SettingsWindow.tsx`、`bridge.ts`、`theme.ts` | 中文设置、实时预览、跨窗事件与串行设置更新 |

没有 `GetCurrentSession()` 单点依赖。优先播放器存在时覆盖自动选择；否则播放状态优先、保持当前正在播放的会话，Windows 当前会话只作后续排序提示。事件即时唤醒，约 1 秒校验一次时间轴，元数据仅在事件/初次读取/15 秒保底更新。UI 的动画帧不调用 Windows API。相同权威采样不会反复重置时钟；新采样会纠偏，暂停冻结，切歌清空旧歌词。异步歌词结果带请求代际编号，迟到的上一首结果不会覆盖新曲。

## 歌词来源与匹配

1. **明确导入的本地覆盖**始终优先（即使是纯文本）。
2. 有效缓存；正缓存 30 天、没有可信匹配的负缓存 10 分钟。网络错误不记负缓存。
3. **LRCLIB** `/api/get`，必要时 `/api/search`。真实 HTTPS、12 秒超时、请求间隔、响应大小上限和 `Retry-After` 限流等待。
4. 网络候选通过匹配后，优先真实逐词时间，其次逐行，再次纯文本；纯音乐单独表示；无结果明确显示未匹配。

标题和艺人做 NFKC / 大小写 / 空白规范化，保留 Live/Remix/Remaster 等录音版本标识。标题、艺人需精确匹配；有时长时容差 2.5 秒，缺少时长时要求专辑一致。置信度为规则评分（非统计概率）：标题/艺人/专辑/时长均一致 1.00，有可靠时长但专辑不同 0.94，缺时长而专辑相同 0.88。拒绝只有模糊相似度的结果。此策略有意偏向漏配，可以通过本地歌词覆盖。

**网易云在线歌词没有启用。** 已实现独立 `netease::Payload` 转换、YRC 解析及翻译对齐，支持本地 YRC；没有引入旧 Binaryify API 项目，没有依赖登录 Cookie、加密签名、未验证的歌曲 ID 或非官方接口。在线适配器下一步必须先解决可靠歌曲 ID 与严格匹配，再以可选 provider 加入。

## 存储与隐私

`%APPDATA%\app.lyricglass.desktop\`：`settings.json` / `.bak`、`themes/`、`local-lyrics/`、`cache/`、`logs/`。设置先写临时文件再替换，保留恢复备份。无账号、遥测、分析、云同步。歌词查询仅发送标题、艺人、专辑和时长，封面不上传；日志记录事件类别、来源进程、误差和失败信息，不记录歌词正文、曲名或凭证。日志最多保留 7 个按天滚动文件。缓存是本地数据，可在退出程序后自行删除。

## 已知边界与验证

本次实际验证结果见 [`docs/VERIFICATION.md`](docs/VERIFICATION.md)。未测试的播放器不能视为已兼容验证。

- 网易云可能延迟发布 seek，延迟期间只能外推；新的权威时间轴到达后立即重新定位。软件不能从缺失的 SMTC 数据推断真实 seek。
- 完全没有时间轴的会话标记为 `timelineAvailable: false`，暂停外推，明确提示无法同步；已导入的歌词降级为静态全文。没有把从软件启动开始计时伪装成真实进度。
- 某些版本/浏览器仅发布标题或没有时长，无法可靠匹配时会要求本地覆盖。复合艺人拼写差异也可能被保守策略拒绝。
- 原生模糊由 Windows 控制强度，不能保证与 CSS 预览半径一一对应；关闭模糊仍可使用透明背景。极大字号/留白需要把悬浮窗调大；不会偷偷缩小用户选定的字体。
- 仅实现 Windows 播放适配器。未实现其他操作系统、播放器登录、音频播放/抓取、流媒体、更新系统或社交功能。
- 歌词取决于 LRCLIB 的覆盖率；不承诺每首歌都有歌词或翻译。在线服务的歌词使用需遵守其条款和权利人的许可。

## 下一阶段

优先解决本机网易云缺失的时间轴，并完成 Spotify 实机回归（暂停、切歌、长时间播放、拖动进度、多显示器 DPI），再接入**可关闭、失败不影响主链路的网易云 YRC 在线 provider**。逐字渲染与本地解析已经具备，下一步的核心是可靠播放位置、歌曲身份与接口可靠性，而非猜测字级时间。

实现基于官方接口文档独立编写，未复制 GPL 项目代码。参考：[Tauri 窗口](https://v2.tauri.app/learn/window-customization/)、[Tauri 全局快捷键](https://v2.tauri.app/plugin/global-shortcut/)、[windows-rs Media.Control](https://microsoft.github.io/windows-docs-rs/doc/windows/Media/Control/index.html)、[LRCLIB API](https://lrclib.net/docs)。
