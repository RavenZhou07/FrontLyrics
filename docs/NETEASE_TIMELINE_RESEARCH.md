# 网易云缺失时间轴：现成方案调研

日期：2026-09-28。此次为源码和文档调研，没有安装插件、重启网易云或开启调试端口，也没有改变应用行为。

## 本机问题

网易云 3.1.41 x64 已开启原生 SMTC，但此前实测歌曲标题、艺人和状态有效，专辑为空，时长和位置为零。增加 SMTC 轮询频率、换歌词服务或导入带时间戳的歌词，均不能提供缺失的真实播放位置。

## 方案一：InfLink-rs 补全 SMTC

- 项目：[apoint123/inflink-rs](https://github.com/apoint123/inflink-rs)。本次核对 main 为 `5a87e0b32aac1a206f8b72a0c70d86a70fb5d736`。
- V3 适配器监听客户端内部 `progressUpdate`、`seekUpdate`；Rust 后端通过 `UpdateTimelineProperties` 发布时间轴。它还补充专辑、歌曲 ID 等元数据。
- 官方安装路径是 BetterNCM 加 InfLink-rs，然后重启网易云。README 声明进度每秒更新一次；本次 releases 页面最新版本为 3.2.11。
- README 明确实测 V3 范围为 3.1.20–3.1.27。manifest 的 `ncm3-compatible` 与 `>=3.1.21` 补丁范围不能证明本机 3.1.41 一定兼容。
- BetterNCM 主仓库已重写并改名 chromatic；作者提供旧代码存档及最后的 BetterNCM 1.3.4 release。不能把 chromatic 当前分支直接当成兼容旧插件的即装即用版本。
- 该项目标记 GPL-3.0；当前 LyricGlass 不复制其源码。作为用户自行选择的外部 SMTC 发布器，可以先通过现有探针验证是否提供了可用数据。

源码证据：[V3 适配器](https://github.com/apoint123/inflink-rs/blob/5a87e0b32aac1a206f8b72a0c70d86a70fb5d736/packages/frontend/src/adapters/v3/adapter.ts)、[SMTC 后端](https://github.com/apoint123/inflink-rs/blob/5a87e0b32aac1a206f8b72a0c70d86a70fb5d736/packages/backend/src/smtc_core.rs)、[插件清单](https://github.com/apoint123/inflink-rs/blob/5a87e0b32aac1a206f8b72a0c70d86a70fb5d736/packages/frontend/manifest.json)、[BetterNCM 维护变更](https://github.com/std-microblock/chromatic)。

## 方案二：参考 Lucent 的本地 CDP 补充适配器

- 项目：[Rex11929282/lucent-updates](https://github.com/Rex11929282/lucent-updates)。本次核对 main 为 `9adcfc5e2b7310ee91f5a6708adc67934c535b17`，LICENSE 为 MIT。
- `electron/ncmcdp.cjs` 连接以调试端口启动的网易云 V3。它尝试监听播放事件，同时读取页面进度条的实际值；源码明确记录了 SMTC 恒为零且旧播放事件不触发的情况。
- 它还能通过 DOM 观察当前高亮歌词。歌词镜像依赖网易云歌词页，不能把镜像一行文字等同于获得完整、可靠的逐词时间轴。
- 该路线不依赖 BetterNCM，但需要开启本地调试接口，并依赖网易云页面/内部接口。未验证本机 3.1.41 是否接受启动参数、最小化后是否继续更新，也未验证暂停、seek 和切歌。
- 调试接口拥有页面控制能力，实施时应只监听本机、明确由用户启用，不接触账号凭证，并支持关闭与断线降级。

源码证据：[CDP 适配器](https://github.com/Rex11929282/lucent-updates/blob/9adcfc5e2b7310ee91f5a6708adc67934c535b17/electron/ncmcdp.cjs)、[进度选择](https://github.com/Rex11929282/lucent-updates/blob/9adcfc5e2b7310ee91f5a6708adc67934c535b17/shared/progressInput.cjs)、[集成说明](https://github.com/Rex11929282/lucent-updates/blob/9adcfc5e2b7310ee91f5a6708adc67934c535b17/docs/PLAYER_INTEGRATION.md)。

## 对 LyricGlass 的建议

保持 SMTC 为默认路径。针对本机先做独立 CDP 兼容性探针，再决定是否增加可选网易云适配器；若用户已有可用的 BetterNCM，则优先试 InfLink-rs 补全标准 SMTC。

通过条件：真实进度递增、暂停冻结、恢复连续、前后 seek 正确、切歌重置、中途启动获取当前进度，以及最小化后的行为。补充数据必须与当前歌曲身份匹配并带新鲜度检查；失联时不得继续宣称精确同步。

完整时间轴恢复后仍需单独验证歌词匹配。YRC 解决歌词内部的逐词时间，不解决播放器当前播放位置；因此应在时间轴验证后再推进。

## 补充：其他歌词项目如何处理（2026-09-28）

本轮读取各仓库当前 HEAD 的相关源码与官方项目文档，未安装或运行这些应用。以下区分源码证据、文档声明和本机验证，不能视为这些项目在本机网易云上已经运行通过。

| 项目 | 核对到的机制 | 对完全缺失时间轴的作用 |
| --- | --- | --- |
| LyricIsland / LyricHover | `TimelineCoordinator` 有可靠/估算/不可用状态；没有可靠位置但正在播放时，以收到的位置建立本地时钟。重复样本继续外推，小幅回退做抑制 | 收到零位置时可以从零计时，但不能知道中途打开或播放器内 seek 后的真实位置；项目也声明网易云拖动进度无法实时同步 |
| MyLyrics | Windows 分支通过 PowerShell 读取 SMTC；`PositionTracker` 获取不到位置时按歌曲开始时间估算，允许点击歌词行校准 | 能提供手动校准的估算模式；其 Windows 函数名虽然包含 UI，实际仍读取 SMTC，不能误认成 Windows 界面进度抓取 |
| LyricsOnTheGo | 读取 SMTC Position 和 LastUpdatedTime，播放时加上采样后经过的时间；优先 Spotify 会话 | 正常 SMTC 的快照外推。核对的读取器没有独立网易云进度来源，不会从零信息中恢复真实位置 |
| Hum | SMTC 加独立播放器桥接；媒体文档与 Windows 入口包含 iTunes、浏览器、Pandora。补充数据有来源优先级与过期规则 | 值得参考多来源仲裁，但本轮核对的媒体入口和说明没有网易云专用进度桥；支持网易云 YRC 歌词源不等于支持网易云客户端真实时间轴 |
| AMLL Player | WebSocket 协议接收歌曲信息、毫秒进度、暂停/恢复等消息；当前 React 消费端将 progress/paused/resumed 更新写入播放状态 | 能绕过 SMTC，但前提是另一个客户端或插件确实能发送真实进度；单独启动接收端不会补出网易云缺失的数据 |
| LyricLens Desktop | 保存多次采样，分类为元数据可用、候选、健康、不稳定等，缺失时改为展开歌词卡片 | 解决降级体验与诊断，不提供新的网易云位置来源 |

### 主要源码与文档

- LyricIsland：[TimelineCoordinator](https://github.com/BochengYao/LyricIsland/blob/HEAD/LyricHover.Core/Media/TimelineCoordinator.cs)、[TimelineMetadataResolver](https://github.com/BochengYao/LyricIsland/blob/HEAD/LyricHover.Core/Media/TimelineMetadataResolver.cs)、[兼容说明](https://github.com/BochengYao/LyricIsland#播放器兼容性)。仅作架构参考，不复制 GPL 代码。
- MyLyrics：[Windows 进度入口](https://github.com/ZiweiZhou99/MyLyrics/blob/HEAD/lib/netease-progress-win32.ts)、[PositionTracker](https://github.com/ZiweiZhou99/MyLyrics/blob/HEAD/lib/position-tracker.ts)。
- LyricsOnTheGo：[SmtcReader](https://github.com/The-MuffinDev/LyricsOnTheGo/blob/HEAD/src/LyricsOnTheGo/Services/SmtcReader.cs)。
- Hum：[媒体与时间规则](https://github.com/basezero-projects/Hum/blob/HEAD/docs/systems/media-and-timing.md)、[Windows 入口](https://github.com/basezero-projects/Hum/blob/HEAD/src-tauri/src/platform/windows/media_backend.rs)、[SMTC](https://github.com/basezero-projects/Hum/blob/HEAD/src-tauri/src/smtc.rs)。
- AMLL Player：[协议](https://github.com/amll-dev/amll-player/blob/HEAD/packages/ws-protocol/README.md)、[接收端](https://github.com/amll-dev/amll-player/blob/HEAD/packages/player/src/components/WSProtocolMusicContext/index.tsx)。
- LyricLens：[时间轴分类与降级](https://github.com/yoruuuchan/lyriclens-desktop/blob/HEAD/src/main.ts)、[SMTC 原始采样](https://github.com/yoruuuchan/lyriclens-desktop/blob/HEAD/src-tauri/src/smtc.rs)。

另查 LyricsBlossom：它的 [README](https://github.com/Eplorr/LyricsBlossom/blob/main/README.md) 明确要求网易云用户安装 BetterNCM + InfLink-rs。本轮获取到的完整仓库文件树只有说明、许可证、截图和发布工作流，没有应用源码，因此这里只将它作为公开使用方案的旁证，不宣称完成源码核查。

### 需要避免照搬的判据

SMTC 的 Position 对应 LastUpdatedTime 时刻，而非每次读取时刻，见 [Microsoft API 定义](https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssessiontimelineproperties?view=winrt-26100)。据此，健康检测不能仅因为几次原始 Position 相同就否定一个有效快照；应同时考虑权威采样时间、播放状态、合理外推及来源能力。

歌词岛还使用 MaxSeekTime 作为 EndTime 缺失时的时长备选。这对部分会话可能有用，但 MaxSeekTime 代表当前最远可跳转位置，不能在所有流媒体场景中直接等同曲目总时长。

### 本机追加只读验证

为排除遗漏字段，使用 Windows PowerShell / WinRT 对 `cloudmusic.exe` 采样三次，间隔一秒，没有改变播放状态。三次均为：

```text
Status = Playing
Position = StartTime = EndTime = MinSeekTime = MaxSeekTime = 0
LastUpdatedTime = 1601-01-01T00:00:00Z（WinRT 零时间戳）
```

因此本机问题属于没有发布有效时间轴，MaxSeekTime 回退无法解决。下一步仍应验证网易云内部进度的 CDP/插件桥接。若增加从切歌或手动校准开始计时的模式，应明确标记为估算，不能将其计为完成自动同步与 seek 验收。
