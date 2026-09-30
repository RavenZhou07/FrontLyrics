export type Mode = "edit" | "locked" | "ghost";
export interface Theme {
  name: string;
  fontFamily: string;
  fontWeight: number;
  fontSize: number;
  lineHeight: number;
  letterSpacing: number;
  align: "left" | "center" | "right";
  activeColor: string;
  inactiveColor: string;
  translationColor: string;
  gradient: boolean;
  gradientColor: string;
  artworkAccent: boolean;
  opacity: number;
  shadow: number;
  glow: number;
  stroke: number;
  strokeColor: string;
  backgroundOpacity: number;
  blur: number;
  radius: number;
  padding: number;
  spacing: number;
  layout: number;
  showTranslation: boolean;
}
export interface Settings {
  version: number;
  theme: Theme;
  mode: Mode;
  overlayVisible: boolean;
  preferredPlayer: string;
  globalOffsetMs: number;
  shortcuts: { toggle: string; earlier: string; later: string };
  geometry: { x: number; y: number; width: number; height: number };
}
export interface Playback {
  sourceId: string;
  sourceApp: string;
  title: string;
  artists: string[];
  album: string;
  durationMs: number;
  positionMs: number;
  status: "playing" | "paused" | "stopped";
  playbackRate: number;
  timelineAvailable: boolean;
  artwork: string | null;
  snapshotAtMs: number;
  trackKey: string;
}
export interface Word {
  startMs: number;
  endMs: number;
  text: string;
}
export interface LyricLine {
  startMs: number;
  endMs: number;
  text: string;
  translation: string | null;
  words: Word[];
}
export interface Lyrics {
  kind: "unavailable" | "plain" | "synced" | "wordTimed" | "instrumental";
  lines: LyricLine[];
  plain: string | null;
  provider: string;
  confidence: number;
  note: string | null;
}
export interface ViewState {
  revision: number;
  settings: Settings;
  playback: Playback | null;
  sessions: { id: string; app: string; title: string; status: string }[];
  lyrics: Lyrics;
  loading: boolean;
  trackOffsetMs: number;
  diagnostic: string | null;
}
