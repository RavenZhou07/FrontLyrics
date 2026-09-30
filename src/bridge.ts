import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import glass from "../themes/glass-mint.json";
import paper from "../themes/paper.json";
import midnight from "../themes/midnight.json";
import type { Settings, Theme, ViewState } from "./types";
export const native = isTauri();
export const builtinThemes = [glass, paper, midnight] as Theme[];
export const defaultSettings: Settings = {
  version: 1,
  theme: glass as Theme,
  mode: "edit",
  overlayVisible: true,
  preferredPlayer: "",
  globalOffsetMs: 0,
  shortcuts: {
    toggle: "Ctrl+Alt+Shift+L",
    earlier: "Ctrl+Alt+Shift+ArrowLeft",
    later: "Ctrl+Alt+Shift+ArrowRight",
  },
  geometry: { x: 240, y: 650, width: 860, height: 230 },
};
export const initialState: ViewState = {
  revision: -1,
  settings: defaultSettings,
  playback: null,
  sessions: [],
  lyrics: {
    kind: "unavailable",
    lines: [],
    plain: null,
    provider: "",
    confidence: 0,
    note: null,
  },
  loading: false,
  trackOffsetMs: 0,
  diagnostic: null,
};
export async function command<T = void>(
  name: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!native) throw new Error("此功能需要 Windows 桌面应用；当前为外观预览。");
  return invoke<T>(name, args);
}
export function useApplication() {
  const [state, setState] = useState(initialState);
  const [draft, setDraft] = useState<Settings | null>(null);
  const [error, setError] = useState("");
  const shown = useRef(state);
  shown.current = { ...state, settings: draft ?? state.settings };
  const pending = useRef<Settings | null>(null),
    saving = useRef(false);
  useEffect(() => {
    if (!native) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const accept = (next: ViewState) => {
      if (!disposed)
        setState((previous) =>
          next.revision >= previous.revision ? next : previous,
        );
    };
    // Subscribe before initial state to avoid losing a track change during startup.
    listen<ViewState>("lyricglass://state", (e) => accept(e.payload))
      .then(async (stop) => {
        if (disposed) {
          stop();
          return;
        }
        unlisten = stop;
        accept(await invoke<ViewState>("get_state"));
      })
      .catch((e) => setError(String(e)));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
  async function flush() {
    if (saving.current) return;
    saving.current = true;
    try {
      while (pending.current) {
        const next = pending.current;
        pending.current = null;
        await invoke("save_settings", { settings: next });
      }
      const next = await invoke<ViewState>("get_state");
      setState((previous) =>
        next.revision >= previous.revision ? next : previous,
      );
      if (!pending.current) setDraft(null);
    } catch (e) {
      pending.current = null;
      setDraft(null);
      setError(String(e));
    } finally {
      saving.current = false;
      if (pending.current) void flush();
    }
  }
  function update(change: (s: Settings) => Settings) {
    const next = change(shown.current.settings);
    shown.current = { ...shown.current, settings: next };
    setDraft(next);
    setError("");
    if (native) {
      pending.current = next;
      void flush();
    } else setState((s) => ({ ...s, settings: next }));
  }
  async function act(name: string, args?: Record<string, unknown>) {
    try {
      setError("");
      await command(name, args);
    } catch (e) {
      setError(String(e));
    }
  }
  return {
    state: { ...state, settings: draft ?? state.settings },
    update,
    act,
    error,
    setError,
  };
}
