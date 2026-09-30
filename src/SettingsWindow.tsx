import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { builtinThemes, command, native } from "./bridge";
import { LyricsView, usePosition } from "./LyricsView";
import { formatTime } from "./timeline";
import { parseTheme } from "./theme";
import type { Settings, Theme, ViewState } from "./types";

type Props = {
  state: ViewState;
  update: (change: (s: Settings) => Settings) => void;
  act: (name: string, args?: Record<string, unknown>) => Promise<void>;
  error: string;
  setError: (value: string) => void;
};
function Field({
  label,
  children,
  hint,
}: {
  label: string;
  children: ReactNode;
  hint?: string;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      {children}
      {hint && <small>{hint}</small>}
    </label>
  );
}
function Toggle({
  label,
  value,
  onChange,
  hint,
}: {
  label: string;
  value: boolean;
  onChange: (v: boolean) => void;
  hint?: string;
}) {
  return (
    <label className="toggle-row">
      <span>
        {label}
        {hint && <small>{hint}</small>}
      </span>
      <input
        type="checkbox"
        role="switch"
        checked={value}
        onChange={(e) => onChange(e.target.checked)}
      />
      <i aria-hidden="true" />
    </label>
  );
}
function Range({
  label,
  value,
  min,
  max,
  step = 1,
  unit = "",
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  unit?: string;
  onChange: (v: number) => void;
}) {
  return (
    <label className="range-field">
      <span>
        {label}
        <output>
          {Number(value.toFixed(2))}
          {unit}
        </output>
      </span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      />
    </label>
  );
}
function Color({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
}) {
  return (
    <label className="color-field">
      <span>{label}</span>
      <div>
        <input
          aria-label={label}
          type="color"
          value={value}
          onChange={(e) => onChange(e.target.value)}
        />
        <code>{value.toUpperCase()}</code>
      </div>
    </label>
  );
}
const kindNames = {
  unavailable: "未匹配",
  plain: "纯文本",
  synced: "逐行同步",
  wordTimed: "逐字同步",
  instrumental: "纯音乐",
};

export function SettingsWindow({ state, update, act, error, setError }: Props) {
  const [tab, setTab] = useState("appearance");
  const [fonts, setFonts] = useState([
    "Segoe UI",
    "Microsoft YaHei UI",
    "Microsoft JhengHei UI",
    "SimHei",
  ]);
  const [themes, setThemes] = useState(builtinThemes);
  const [themeName, setThemeName] = useState("我的主题");
  const [shortcuts, setShortcuts] = useState(state.settings.shortcuts);
  const [preview, setPreview] = useState(true);
  const themeInput = useRef<HTMLInputElement>(null),
    lyricsInput = useRef<HTMLInputElement>(null);
  const importTrack = useRef("");
  const { settings, playback, lyrics } = state,
    t = settings.theme;
  const position = usePosition(playback);
  useEffect(() => {
    setShortcuts(settings.shortcuts);
  }, [
    settings.shortcuts.toggle,
    settings.shortcuts.earlier,
    settings.shortcuts.later,
  ]);
  useEffect(() => {
    if (!native) return;
    command<string[]>("list_fonts")
      .then(setFonts)
      .catch((e) => setError(String(e)));
    command<Theme[]>("list_themes")
      .then(setThemes)
      .catch((e) => setError(String(e)));
  }, [setError]);
  function style<K extends keyof Theme>(key: K, value: Theme[K]) {
    update((s) => ({ ...s, theme: { ...s.theme, [key]: value } }));
  }
  async function saveTheme() {
    try {
      const theme = parseTheme(JSON.stringify({ ...t, name: themeName }));
      if (native) await command("save_theme", { theme });
      setThemes((all) => [...all.filter((v) => v.name !== theme.name), theme]);
      style("name", theme.name);
      setError("");
    } catch (e) {
      setError(String(e));
    }
  }
  async function exportTheme() {
    if (native) {
      await act("export_theme", { theme: t });
      return;
    }
    const url = URL.createObjectURL(
      new Blob([JSON.stringify(t, null, 2)], { type: "application/json" }),
    );
    const a = document.createElement("a");
    a.href = url;
    a.download = `${t.name}.json`;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
  return (
    <div className="settings-app">
      <aside className="sidebar">
        <div className="brand">
          <img src="/logo.svg" alt="" />
          <div>
            LyricGlass<small>音乐之外，歌词之上</small>
          </div>
        </div>
        <div className="nav-caption">你的歌词空间</div>
        <nav aria-label="设置分类">
          {[
            ["appearance", "Aa", "外观与排版"],
            ["lyrics", "≋", "歌词与同步"],
            ["behavior", "⌘", "窗口与快捷键"],
          ].map(([id, icon, label]) => (
            <button
              key={id}
              className={tab === id ? "selected" : ""}
              onClick={() => setTab(id)}
            >
              <b>{icon}</b>
              {label}
              <span>›</span>
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <span
            className={`status-dot ${playback?.status === "playing" ? "playing" : ""}`}
          />
          <span>
            {native ? "Windows 媒体会话" : "浏览器外观预览"}
            <small>
              {playback
                ? `已连接 · ${playback.sourceApp}`
                : native
                  ? "等待播放器"
                  : "在桌面端连接音乐"}
            </small>
          </span>
        </div>
        <div className="version">
          LYRICGLASS <span>0.1.0</span>
        </div>
      </aside>
      <main className="settings-main">
        <header className="page-header">
          <div>
            <p className="eyebrow">MADE FOR YOUR DESKTOP</p>
            <h1>
              {tab === "appearance"
                ? "让歌词成为你的风格。"
                : tab === "lyrics"
                  ? "和每一拍，恰好同步。"
                  : "安静地，留在你需要的地方。"}
            </h1>
            <p>
              {tab === "appearance"
                ? "从一个字的形状，到整句的呼吸感。每一次调整，即刻呈现。"
                : tab === "lyrics"
                  ? "跟随正在播放的音乐，优先展示有把握的匹配。"
                  : "选择交互方式，用熟悉的按键调整你的歌词空间。"}
            </p>
          </div>
          <button
            className={`visibility-button ${settings.overlayVisible ? "on" : ""}`}
            onClick={() =>
              update((s) => ({ ...s, overlayVisible: !s.overlayVisible }))
            }
          >
            <span className="status-dot" />
            {settings.overlayVisible ? "悬浮窗已开启" : "悬浮窗已隐藏"}
          </button>
        </header>
        {(error || state.diagnostic) && (
          <div className="notice error" role="alert">
            {error || state.diagnostic}
            <button onClick={() => setError("")}>×</button>
          </div>
        )}
        {!native && (
          <div className="notice">
            当前是浏览器中的外观预览。运行 <code>npm run dev</code>{" "}
            以启用真实媒体检测和桌面悬浮窗。
          </div>
        )}
        {playback && !playback.timelineAvailable && (
          <div className="notice" role="status">
            已连接 {playback.sourceApp}
            ，但此会话没有提供时长和播放进度，无法可靠同步。可导入本地歌词静态阅读；不会将软件运行时间当作歌曲进度。
          </div>
        )}
        <section className="preview-card">
          <div className="section-top">
            <span className="eyebrow">
              LIVE CANVAS <i /> 即时预览
            </span>
            <div className="segmented small">
              <button
                className={preview ? "active" : ""}
                onClick={() => setPreview(true)}
              >
                示例文字
              </button>
              <button
                className={!preview ? "active" : ""}
                onClick={() => setPreview(false)}
              >
                正在播放
              </button>
            </div>
          </div>
          <div className="preview-stage">
            <div className="preview-grain" />
            <LyricsView
              theme={t}
              lyrics={lyrics}
              artwork={playback?.artwork}
              timelineAvailable={playback?.timelineAvailable ?? true}
              position={
                position + settings.globalOffsetMs + state.trackOffsetMs
              }
              preview={preview}
              emptyLabel={playback ? "暂无匹配歌词" : "播放音乐，歌词即刻浮现"}
            />
          </div>
          <div className="preview-footer">
            <span>
              {preview
                ? "原创示例文案 · 仅用于外观预览"
                : playback
                  ? `${playback.sourceApp} · ${playback.title}`
                  : "打开 Spotify 或网易云音乐，播放一首歌"}
            </span>
            <span>
              {t.fontFamily} / {t.fontSize}px
            </span>
          </div>
        </section>
        {tab === "appearance" && (
          <>
            <section className="settings-section">
              <div className="section-heading">
                <div>
                  <h2>从一个主题开始</h2>
                  <p>只是起点。所有细节，都可以继续调整。</p>
                </div>
                <div className="button-row">
                  <button onClick={() => themeInput.current?.click()}>
                    导入 JSON
                  </button>
                  <button onClick={exportTheme}>导出主题 ↗</button>
                </div>
              </div>
              <div className="theme-grid">
                {themes.map((theme, i) => (
                  <button
                    key={`${theme.name}-${i}`}
                    className={`theme-card ${t.name === theme.name ? "chosen" : ""}`}
                    onClick={() => update((s) => ({ ...s, theme }))}
                  >
                    <span
                      className="theme-sample"
                      style={{
                        color: theme.activeColor,
                        fontFamily: theme.fontFamily,
                      }}
                    >
                      每一句 <em>Aa</em>
                    </span>
                    <span>
                      {theme.name}
                      <i>{t.name === theme.name ? "✓" : "↗"}</i>
                    </span>
                  </button>
                ))}
              </div>
              <div className="save-theme">
                <input
                  aria-label="主题名称"
                  maxLength={80}
                  value={themeName}
                  onChange={(e) => setThemeName(e.target.value)}
                />
                <button onClick={saveTheme}>保存为新主题</button>
              </div>
            </section>
            <div className="panels">
              <section className="panel">
                <h2>
                  字体与排版 <span>01</span>
                </h2>
                <Field
                  label="字体家族"
                  hint="自动回退到系统中文字体，支持输入已安装字体名称。"
                >
                  <input
                    list="system-fonts"
                    value={t.fontFamily}
                    onChange={(e) => {
                      if (e.target.value) style("fontFamily", e.target.value);
                    }}
                  />
                  <datalist id="system-fonts">
                    {fonts.map((f) => (
                      <option key={f} value={f} />
                    ))}
                  </datalist>
                </Field>
                <div className="two-columns">
                  <Field label="字重">
                    <select
                      value={t.fontWeight}
                      onChange={(e) =>
                        style("fontWeight", Number(e.target.value))
                      }
                    >
                      {[100, 200, 300, 400, 500, 600, 700, 800, 900].map((w) => (
                        <option key={w} value={w}>
                          {w}
                          {w === 600 ? " · 半粗" : ""}
                        </option>
                      ))}
                    </select>
                  </Field>
                  <Field label="对齐">
                    <select
                      value={t.align}
                      onChange={(e) =>
                        style("align", e.target.value as Theme["align"])
                      }
                    >
                      <option value="left">左对齐</option>
                      <option value="center">居中</option>
                      <option value="right">右对齐</option>
                    </select>
                  </Field>
                </div>
                <Range
                  label="字号"
                  value={t.fontSize}
                  min={12}
                  max={120}
                  unit=" px"
                  onChange={(v) => style("fontSize", v)}
                />
                <Range
                  label="行高"
                  value={t.lineHeight}
                  min={1}
                  max={2.5}
                  step={0.05}
                  onChange={(v) => style("lineHeight", v)}
                />
                <Range
                  label="字间距"
                  value={t.letterSpacing}
                  min={-2}
                  max={12}
                  step={0.1}
                  unit=" px"
                  onChange={(v) => style("letterSpacing", v)}
                />
                <Field label="显示行数">
                  <div className="segmented">
                    {[1, 2, 3].map((n) => (
                      <button
                        key={n}
                        className={t.layout === n ? "active" : ""}
                        onClick={() => style("layout", n)}
                      >
                        {["单行", "双行", "三行"][n - 1]}
                      </button>
                    ))}
                  </div>
                </Field>
                <Toggle
                  label="显示翻译"
                  value={t.showTranslation}
                  onChange={(v) => style("showTranslation", v)}
                  hint="有翻译时显示在当前歌词下方"
                />
              </section>
              <section className="panel">
                <h2>
                  色彩与光感 <span>02</span>
                </h2>
                <Color
                  label="当前歌词"
                  value={t.activeColor}
                  onChange={(v) => style("activeColor", v)}
                />
                <Color
                  label="前后歌词"
                  value={t.inactiveColor}
                  onChange={(v) => style("inactiveColor", v)}
                />
                <Color
                  label="翻译文字"
                  value={t.translationColor}
                  onChange={(v) => style("translationColor", v)}
                />
                <Toggle
                  label="双颜色渐变"
                  value={t.gradient}
                  onChange={(v) => style("gradient", v)}
                />
                {t.gradient && (
                  <Color
                    label="渐变终点"
                    value={t.gradientColor}
                    onChange={(v) => style("gradientColor", v)}
                  />
                )}
                <Toggle
                  label="从专辑封面取色"
                  value={t.artworkAccent}
                  onChange={(v) => style("artworkAccent", v)}
                  hint="仅改变当前歌词主色；没有封面时使用自选颜色"
                />
                <Range
                  label="整体不透明度"
                  value={t.opacity}
                  min={0.05}
                  max={1}
                  step={0.01}
                  onChange={(v) => style("opacity", v)}
                />
                <Range
                  label="文字阴影"
                  value={t.shadow}
                  min={0}
                  max={40}
                  unit=" px"
                  onChange={(v) => style("shadow", v)}
                />
                <Range
                  label="柔光"
                  value={t.glow}
                  min={0}
                  max={40}
                  unit=" px"
                  onChange={(v) => style("glow", v)}
                />
                <Range
                  label="描边"
                  value={t.stroke}
                  min={0}
                  max={4}
                  step={0.1}
                  unit=" px"
                  onChange={(v) => style("stroke", v)}
                />
                {t.stroke > 0 && (
                  <Color
                    label="描边颜色"
                    value={t.strokeColor}
                    onChange={(v) => style("strokeColor", v)}
                  />
                )}
              </section>
            </div>
            <section className="panel surface-panel">
              <h2>
                留白与底色 <span>03</span>
              </h2>
              <div className="three-columns">
                <Range
                  label="背景不透明度"
                  value={t.backgroundOpacity}
                  min={0}
                  max={1}
                  step={0.01}
                  onChange={(v) => style("backgroundOpacity", v)}
                />
                <Range
                  label="磨砂模糊"
                  value={t.blur}
                  min={0}
                  max={60}
                  unit=" px"
                  onChange={(v) => style("blur", v)}
                />
                <Range
                  label="圆角"
                  value={t.radius}
                  min={0}
                  max={80}
                  unit=" px"
                  onChange={(v) => style("radius", v)}
                />
                <Range
                  label="内边距"
                  value={t.padding}
                  min={0}
                  max={100}
                  unit=" px"
                  onChange={(v) => style("padding", v)}
                />
                <Range
                  label="歌词间距"
                  value={t.spacing}
                  min={0}
                  max={60}
                  unit=" px"
                  onChange={(v) => style("spacing", v)}
                />
              </div>
              <p className="helper">
                原生桌面磨砂效果由 Windows
                版本决定；半径在预览中精确呈现，系统桌面模糊强度由系统控制。
              </p>
            </section>
          </>
        )}
        {tab === "lyrics" && (
          <>
            <section className="panel now-playing">
              <div className="album-art">
                {playback?.artwork ? (
                  <img src={playback.artwork} alt="专辑封面" />
                ) : (
                  <span>♪</span>
                )}
              </div>
              <div>
                <p className="eyebrow">
                  {playback?.sourceApp ?? "WAITING FOR MUSIC"}
                </p>
                <h2>{playback?.title ?? "还没有正在播放的音乐"}</h2>
                <p>
                  {playback?.artists.join(" / ") ||
                    "打开一个支持 Windows 媒体会话的播放器"}
                </p>
                <span className="metadata">
                  {formatTime(position)} /{" "}
                  {formatTime(playback?.durationMs ?? 0)} ·{" "}
                  {playback?.status === "playing"
                    ? "播放中"
                    : playback?.status === "paused"
                      ? "已暂停"
                      : "待机"}
                </span>
              </div>
            </section>
            <div className="panels">
              <section className="panel">
                <h2>歌词来源</h2>
                <div className="provider-status">
                  <b>
                    {state.loading
                      ? "正在查找…"
                      : lyrics.provider || "等待匹配"}
                  </b>
                  <span className="pill">{kindNames[lyrics.kind]}</span>
                </div>
                <p className="helper">
                  {lyrics.confidence > 0
                    ? `匹配置信度 ${(lyrics.confidence * 100).toFixed(0)}%`
                    : lyrics.note || "播放后自动查询 LRCLIB。"}
                </p>
                <p className="helper">
                  本地覆盖 → 本地缓存 →
                  LRCLIB。网络结果优先逐字，其次逐行，最后纯文本。
                </p>
                <div className="button-row">
                  <button
                    disabled={!playback || state.loading}
                    onClick={() => act("refresh_lyrics")}
                  >
                    重新匹配
                  </button>
                  <button
                    disabled={!playback}
                    onClick={() => {
                      importTrack.current = playback?.trackKey ?? "";
                      lyricsInput.current?.click();
                    }}
                  >
                    导入 LRC / YRC
                  </button>
                </div>
                {lyrics.provider === "Local" && (
                  <button
                    className="text-button"
                    onClick={() => act("remove_local_lyrics")}
                  >
                    移除本曲本地覆盖
                  </button>
                )}
                <p className="helper">
                  纯文本会完整展示，不模拟同步。支持 UTF-8 歌词文件。
                </p>
              </section>
              <section className="panel">
                <h2>同步微调</h2>
                <Field
                  label="全局偏移（毫秒）"
                  hint="正值让歌词提前，负值让歌词延后。"
                >
                  <input
                    type="number"
                    min={-600000}
                    max={600000}
                    step={100}
                    value={settings.globalOffsetMs}
                    onChange={(e) =>
                      update((s) => ({
                        ...s,
                        globalOffsetMs: Math.max(
                          -600000,
                          Math.min(600000, Number(e.target.value)),
                        ),
                      }))
                    }
                  />
                </Field>
                <Field
                  label="仅当前歌曲（毫秒）"
                  hint="切歌后自动归零；快捷键每次调整 100 ms。"
                >
                  <input
                    type="number"
                    min={-600000}
                    max={600000}
                    step={100}
                    value={state.trackOffsetMs}
                    onChange={(e) =>
                      act("set_track_offset", {
                        offset: Number(e.target.value),
                      })
                    }
                  />
                </Field>
                <button onClick={() => act("set_track_offset", { offset: 0 })}>
                  重置本曲偏移
                </button>
                <p className="helper">
                  网易云的拖动进度有时延迟上报。收到新的 Windows
                  时间轴后，歌词会自动重新对齐。
                </p>
              </section>
            </div>
            <section className="panel">
              <h2>播放器与会话</h2>
              <Field label="优先跟随">
                <select
                  value={settings.preferredPlayer}
                  onChange={(e) =>
                    update((s) => ({ ...s, preferredPlayer: e.target.value }))
                  }
                >
                  <option value="">自动 · 优先正在播放的会话</option>
                  {Array.from(
                    new Set([
                      "Spotify",
                      "网易云音乐",
                      "QQ 音乐",
                      "Apple Music",
                      "YouTube Music",
                      ...state.sessions.map((s) => s.app),
                    ]),
                  ).map((app) => (
                    <option key={app}>{app}</option>
                  ))}
                </select>
              </Field>
              <p className="helper">
                选定播放器存在时优先跟随（包括暂停状态）。否则自动选择；不会让暂停的浏览器标签页抢占正在播放的音乐。
              </p>
              {state.sessions.length ? (
                state.sessions.map((s) => (
                  <div className="session-row" key={s.id}>
                    <span
                      className={`status-dot ${s.status === "playing" ? "playing" : ""}`}
                    />
                    <b>{s.app}</b>
                    <span>{s.title || "无曲目元数据"}</span>
                    <small>{s.status}</small>
                  </div>
                ))
              ) : (
                <div className="empty-state">
                  尚未发现 Windows 媒体会话。播放器需启用系统媒体集成。
                </div>
              )}
            </section>
          </>
        )}
        {tab === "behavior" && (
          <>
            <section className="panel">
              <h2>悬浮窗交互</h2>
              <div className="mode-grid">
                {(
                  [
                    ["edit", "编辑", "拖动、调整大小，找到舒服的位置。"],
                    ["locked", "锁定", "固定位置，双击歌词打开设置。"],
                    ["ghost", "穿透", "鼠标穿过歌词，不打断桌面操作。"],
                  ] as const
                ).map(([mode, title, desc]) => (
                  <button
                    key={mode}
                    className={settings.mode === mode ? "chosen" : ""}
                    onClick={() => update((s) => ({ ...s, mode }))}
                  >
                    <strong>
                      {title}
                      <span>{settings.mode === mode ? "✓" : "○"}</span>
                    </strong>
                    <p>{desc}</p>
                  </button>
                ))}
              </div>
              <p className="helper">
                穿透模式下，可通过托盘菜单或全局快捷键返回编辑模式。关闭设置窗口后，程序会继续在托盘运行。
              </p>
              <button onClick={() => act("reset_overlay_position")}>
                找回悬浮窗 · 居中并恢复尺寸
              </button>
            </section>
            <section className="panel">
              <h2>全局快捷键</h2>
              <div className="three-columns">
                {(
                  [
                    ["toggle", "切换交互模式"],
                    ["earlier", "歌词提前 100 ms"],
                    ["later", "歌词延后 100 ms"],
                  ] as const
                ).map(([key, label]) => (
                  <Field key={key} label={label}>
                    <input
                      value={shortcuts[key]}
                      onChange={(e) =>
                        setShortcuts((s) => ({ ...s, [key]: e.target.value }))
                      }
                    />
                  </Field>
                ))}
              </div>
              <div className="button-row">
                <button
                  className="primary"
                  onClick={() => update((s) => ({ ...s, shortcuts }))}
                >
                  应用快捷键
                </button>
                <span className="helper">
                  格式示例：Ctrl+Alt+Shift+L。冲突会显示错误并保留原绑定。
                </span>
              </div>
            </section>
            <section className="panel privacy">
              <span>◈</span>
              <div>
                <h2>音乐留在你的电脑上。</h2>
                <p>
                  无需登录，无遥测，无播放历史上传。歌词查询仅发送标题、艺人、专辑与时长。封面只在本地读取与取色。
                </p>
                <p>
                  设置、主题、本地歌词和缓存保存在
                  %APPDATA%\app.lyricglass.desktop；诊断日志保留最近 7
                  个文件，不记录歌曲标题。
                </p>
              </div>
            </section>
          </>
        )}
        <footer className="page-footer">
          <span>把桌面留给音乐。</span>
          <span>
            {native ? "设置自动保存在本机" : "外观预览 · 设置仅在本页生效"}{" "}
            <i>✓</i>
          </span>
        </footer>
      </main>
      <input
        type="file"
        hidden
        ref={themeInput}
        accept=".json,application/json"
        onChange={async (e) => {
          const file = e.target.files?.[0];
          e.target.value = "";
          if (!file) return;
          try {
            if (file.size > 50000) throw new Error("主题文件过大");
            const theme = parseTheme(await file.text());
            if (native) await command("save_theme", { theme });
            setThemes((all) => [
              ...all.filter((v) => v.name !== theme.name),
              theme,
            ]);
            update((s) => ({ ...s, theme }));
            setError("");
          } catch (err) {
            setError(String(err));
          }
        }}
      />
      <input
        type="file"
        hidden
        ref={lyricsInput}
        accept=".lrc,.yrc,.txt"
        onChange={async (e) => {
          const file = e.target.files?.[0];
          e.target.value = "";
          if (!file) return;
          if (file.size > 2000000) {
            setError("歌词文件不能超过 2 MB");
            return;
          }
          try {
            await act("import_lyrics", {
              trackKey: importTrack.current,
              text: await file.text(),
            });
          } catch (err) {
            setError(String(err));
          }
        }}
      />
    </div>
  );
}
