import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ViewState } from "./types";
import { LyricsView, usePosition } from "./LyricsView";
import { native } from "./bridge";
export function Overlay({
  state,
  act,
  error,
}: {
  state: ViewState;
  act: (name: string, args?: Record<string, unknown>) => Promise<void>;
  error: string;
}) {
  const { settings, playback, lyrics } = state;
  const pos =
    usePosition(playback) + settings.globalOffsetMs + state.trackOffsetMs;
  const edit = settings.mode === "edit";
  return (
    <main
      className={`overlay-window ${edit ? "editing" : ""}`}
      onDoubleClick={() => {
        if (settings.mode !== "ghost") void act("open_settings");
      }}
      onPointerDown={(e) => {
        if (
          edit &&
          e.button === 0 &&
          !(e.target as HTMLElement).closest("button")
        ) {
          e.preventDefault();
          if (native)
            void getCurrentWindow()
              .startDragging()
              .catch(() => undefined);
        }
      }}
    >
      {edit && (
        <div className="overlay-toolbar">
          <span>⠿ &nbsp; 拖动歌词</span>
          <div>
            <button onClick={() => act("open_settings")}>设置</button>
            <button onClick={() => act("set_mode", { mode: "ghost" })}>
              穿透
            </button>
          </div>
        </div>
      )}
      <LyricsView
        theme={settings.theme}
        lyrics={lyrics}
        position={pos}
        artwork={playback?.artwork}
        timelineAvailable={playback?.timelineAvailable ?? true}
        emptyLabel={
          state.loading
            ? "正在寻找歌词…"
            : playback && !playback.timelineAvailable
              ? "播放器未提供时间轴"
              : playback
                ? "暂无匹配歌词"
                : "播放音乐，歌词即刻浮现"
        }
      />
      {edit && (
        <button
          className="resize-grip"
          aria-label="调整歌词窗口大小"
          onPointerDown={(e) => {
            e.stopPropagation();
            e.preventDefault();
            if (native)
              void getCurrentWindow()
                .startResizeDragging("SouthEast")
                .catch(() => undefined);
          }}
        >
          ⌟
        </button>
      )}
      {error && edit && <div className="overlay-error">{error}</div>}
    </main>
  );
}
