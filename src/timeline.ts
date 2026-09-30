import type { LyricLine, Playback, Word } from "./types";
export function positionAt(p: Playback | null, elapsedMs: number): number {
  if (!p) return 0;
  const pos = Math.max(
    0,
    p.positionMs +
      (p.status === "playing" && p.timelineAvailable
        ? Math.max(0, elapsedMs) * p.playbackRate
        : 0),
  );
  return p.durationMs > 0 ? Math.min(pos, p.durationMs) : pos;
}
export function activeLine(lines: LyricLine[], position: number): number {
  let low = 0,
    high = lines.length;
  while (low < high) {
    const mid = (low + high) >>> 1;
    if (lines[mid].startMs <= position) low = mid + 1;
    else high = mid;
  }
  const index = low - 1;
  return index >= 0 && position < lines[index].endMs ? index : -1;
}
export function wordProgress(word: Word, position: number): number {
  return Math.max(
    0,
    Math.min(
      1,
      (position - word.startMs) / Math.max(1, word.endMs - word.startMs),
    ),
  );
}
export function formatTime(ms: number): string {
  const s = Math.floor(Math.max(ms, 0) / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}
