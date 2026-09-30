import { expect, it } from "vitest";
import { parseTheme } from "./theme";
it("loads partial readable presets with defaults and preserves CJK fonts", () => {
  const theme = parseTheme('{"name":"中文主题","fontFamily":"微软雅黑"}');
  expect(theme.fontFamily).toBe("微软雅黑");
  expect(theme.fontSize).toBe(34);
  expect(parseTheme(JSON.stringify(theme))).toEqual(theme);
});
it("rejects untrusted malformed theme values before applying", () => {
  for (const value of [
    "[]",
    "null",
    '{"fontSize":999}',
    '{"activeColor":"red"}',
    '{"gradient":"false"}',
    '{"layout":1.5}',
    '{"align":"up"}',
  ])
    expect(() => parseTheme(value)).toThrow();
});
