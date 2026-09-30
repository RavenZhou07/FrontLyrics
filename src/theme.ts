import type { Theme } from "./types";
import { builtinThemes } from "./bridge";
const ranges: Partial<Record<keyof Theme, [number, number]>> = {
  fontSize: [12, 120],
  fontWeight: [100, 900],
  lineHeight: [1, 2.5],
  letterSpacing: [-2, 12],
  opacity: [0.05, 1],
  shadow: [0, 40],
  glow: [0, 40],
  stroke: [0, 4],
  backgroundOpacity: [0, 1],
  blur: [0, 60],
  radius: [0, 80],
  padding: [0, 100],
  spacing: [0, 60],
  layout: [1, 3],
};
export function parseTheme(text: string): Theme {
  const value: unknown = JSON.parse(text);
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error("主题必须是 JSON 对象");
  const theme = { ...builtinThemes[0] };
  for (const key of Object.keys(theme) as (keyof Theme)[]) {
    if (!(key in value)) continue;
    const field = (value as Record<string, unknown>)[key];
    if (typeof field !== typeof theme[key])
      throw new Error(`主题字段类型无效：${key}`);
    const range = ranges[key];
    if (
      range &&
      (typeof field !== "number" ||
        !Number.isFinite(field) ||
        field < range[0] ||
        field > range[1])
    )
      throw new Error(`主题数值超出范围：${key}`);
    if (
      key.endsWith("Color") &&
      (typeof field !== "string" || !/^#[\da-f]{6}$/i.test(field))
    )
      throw new Error(`颜色格式无效：${key}`);
    Object.assign(theme, { [key]: field });
  }
  if (
    !["left", "center", "right"].includes(theme.align) ||
    ![1, 2, 3].includes(theme.layout) ||
    !Number.isInteger(theme.fontWeight)
  )
    throw new Error("排版参数无效");
  if (
    !theme.fontFamily ||
    theme.fontFamily.length > 150 ||
    !theme.name.trim() ||
    theme.name.length > 80
  )
    throw new Error("字体或主题名称无效");
  return theme;
}
