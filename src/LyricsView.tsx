import { useEffect, useMemo, useState } from "react";
import type { CSSProperties } from "react";
import type { Lyrics, LyricLine, Playback, Theme } from "./types";
import { activeLine, positionAt, wordProgress } from "./timeline";

export function usePosition(playback: Playback | null): number {
  const anchor = useMemo(
    () => ({ playback, received: performance.now() }),
    [playback],
  );
  const [position, setPosition] = useState(0);
  useEffect(() => {
    let id = 0;
    const tick = () => {
      setPosition(
        positionAt(anchor.playback, performance.now() - anchor.received),
      );
      id = requestAnimationFrame(tick);
    };
    tick();
    return () => cancelAnimationFrame(id);
  }, [anchor]);
  return position;
}
function useArtworkAccent(artwork?: string | null): string | null {
  const [accent, setAccent] = useState<string | null>(null);
  useEffect(() => {
    setAccent(null);
    if (!artwork?.startsWith("data:image/")) return;
    let cancelled = false;
    const img = new Image();
    img.onload = () => {
      const canvas = document.createElement("canvas");
      canvas.width = canvas.height = 12;
      const ctx = canvas.getContext("2d");
      if (!ctx || cancelled) return;
      ctx.drawImage(img, 0, 0, 12, 12);
      const data = ctx.getImageData(0, 0, 12, 12).data;
      let r = 0,
        g = 0,
        b = 0,
        n = 0;
      for (let i = 0; i < data.length; i += 4) {
        if (data[i + 3] < 100) continue;
        r += data[i];
        g += data[i + 1];
        b += data[i + 2];
        n++;
      }
      if (n)
        setAccent(
          `rgb(${[r, g, b].map((c) => Math.round((c / n) * 0.45 + 140)).join(",")})`,
        );
    };
    img.src = artwork;
    return () => {
      cancelled = true;
    };
  }, [artwork]);
  return accent;
}
function Line({
  line,
  position,
  active,
  theme,
}: {
  line: LyricLine;
  position: number;
  active: boolean;
  theme: Theme;
}) {
  return (
    <div className={`lyric-line ${active ? "current" : "inactive"}`}>
      <div
        className={
          active && theme.gradient && !line.words.length ? "gradient-text" : ""
        }
      >
        {active && line.words.length
          ? line.words.map((w, i) => (
              <span className="word" key={`${w.startMs}-${i}`}>
                <span className="word-base">{w.text}</span>
                <span
                  aria-hidden="true"
                  className={`word-fill ${theme.gradient ? "gradient-text" : ""}`}
                  style={{
                    clipPath: `inset(0 ${(1 - wordProgress(w, position)) * 100}% 0 0)`,
                  }}
                >
                  {w.text}
                </span>
              </span>
            ))
          : line.text || "\u00a0"}
      </div>
      {active && theme.showTranslation && line.translation && (
        <div className="translation">{line.translation}</div>
      )}
    </div>
  );
}
const exampleLines: LyricLine[] = [
  {
    startMs: 0,
    endMs: 2000,
    text: "让旋律，留在此刻",
    translation: null,
    words: [],
  },
  {
    startMs: 2000,
    endMs: 6000,
    text: "每一句，都有自己的光",
    translation: "Every word, in your own light.",
    words: [],
  },
  {
    startMs: 6000,
    endMs: 9000,
    text: "听见音乐，也看见自己",
    translation: null,
    words: [],
  },
];
export function LyricsView({
  theme,
  lyrics,
  position,
  artwork,
  preview = false,
  timelineAvailable = true,
  emptyLabel = "等待音乐",
}: {
  theme: Theme;
  lyrics: Lyrics;
  position: number;
  artwork?: string | null;
  preview?: boolean;
  timelineAvailable?: boolean;
  emptyLabel?: string;
}) {
  const accent = useArtworkAccent(theme.artworkAccent ? artwork : null);
  const lines = preview ? exampleLines : lyrics.lines;
  const index = preview ? 1 : activeLine(lines, position);
  const active = accent ?? theme.activeColor;
  const style = {
    fontFamily: `"${theme.fontFamily.replace(/["\\]/g, "")}", "Microsoft YaHei UI", "Microsoft JhengHei UI", "Noto Sans CJK SC", sans-serif`,
    fontSize: theme.fontSize,
    fontWeight: theme.fontWeight,
    lineHeight: theme.lineHeight,
    letterSpacing: theme.letterSpacing,
    textAlign: theme.align,
    opacity: theme.opacity,
    backgroundColor: `rgba(9, 17, 16, ${theme.backgroundOpacity})`,
    backdropFilter: theme.blur ? `blur(${theme.blur}px)` : undefined,
    borderRadius: theme.radius,
    padding: theme.padding,
    "--active": active,
    "--inactive": theme.inactiveColor,
    "--translation": theme.translationColor,
    "--gradient": theme.gradientColor,
    "--spacing": `${theme.spacing}px`,
    textShadow: `0 2px ${theme.shadow}px rgba(0,0,0,${theme.shadow ? 0.8 : 0}), 0 0 ${theme.glow}px ${theme.glow ? active : "transparent"}`,
    WebkitTextStroke: `${theme.stroke}px ${theme.strokeColor}`,
  } as CSSProperties;
  if (
    !preview &&
    (lyrics.kind === "plain" || (!timelineAvailable && lines.length > 0))
  )
    return (
      <div className="lyrics-surface plain-lyrics" style={style}>
        <small>
          {timelineAvailable
            ? "纯文本 · 无同步时间轴"
            : "播放器未提供时间轴 · 静态歌词"}
        </small>
        <div>{lyrics.plain || lines.map((l) => l.text).join("\n")}</div>
      </div>
    );
  const indices =
    theme.layout === 1
      ? [index]
      : theme.layout === 2
        ? [index, index + 1]
        : [index - 1, index, index + 1];
  return (
    <div className={`lyrics-surface layout-${theme.layout}`} style={style}>
      {index < 0 ? (
        <div className="lyric-line current empty-lyric">
          {!preview && lyrics.kind === "instrumental"
            ? "♪  纯音乐"
            : lyrics.kind === "synced" || lyrics.kind === "wordTimed"
              ? "♪"
              : emptyLabel}
        </div>
      ) : (
        indices.map((i, slot) =>
          lines[i] ? (
            <div className="line-slot" key={`${i}-${lines[i].startMs}`}>
              <Line
                line={lines[i]}
                position={position}
                active={i === index}
                theme={theme}
              />
            </div>
          ) : (
            <div className="line-slot spacer" key={`spacer-${slot}`}>
              　
            </div>
          ),
        )
      )}
    </div>
  );
}
