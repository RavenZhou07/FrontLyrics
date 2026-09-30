import { describe, expect, it } from "vitest";
import { activeLine, positionAt, wordProgress } from "./timeline";
import type { Playback, LyricLine } from "./types";
describe("renderer timeline", () => {
  it("extrapolates playing, freezes paused, clamps at duration", () => {
    const p = {
      positionMs: 1000,
      durationMs: 5000,
      playbackRate: 1,
      status: "playing",
      timelineAvailable: true,
    } as Playback;
    expect(positionAt(p, 500)).toBe(1500);
    expect(positionAt({ ...p, status: "paused" }, 500)).toBe(1000);
    expect(positionAt(p, 9000)).toBe(5000);
    expect(positionAt(null, 100)).toBe(0);
    expect(positionAt({ ...p, timelineAvailable: false }, 10000)).toBe(1000);
  });
  it("honors intro, boundaries, gaps, empty separators and seek backwards", () => {
    const lines = [
      { startMs: 1000, endMs: 2000, text: "one" },
      { startMs: 3000, endMs: 4000, text: "" },
    ] as LyricLine[];
    expect(activeLine(lines, 0)).toBe(-1);
    expect(activeLine(lines, 1000)).toBe(0);
    expect(activeLine(lines, 2500)).toBe(-1);
    expect(activeLine(lines, 3000)).toBe(1);
    expect(activeLine(lines, 4000)).toBe(-1);
    expect(activeLine(lines, 1500)).toBe(0);
  });
  it("highlights only real word timing", () => {
    const w = { text: "你好", startMs: 1000, endMs: 2000 };
    expect(wordProgress(w, 0)).toBe(0);
    expect(wordProgress(w, 1500)).toBe(0.5);
    expect(wordProgress(w, 9000)).toBe(1);
  });
});
