import type { LauncherTheme } from "@/lib/generated/LauncherTheme";

export function isLauncherTheme(value: unknown): value is LauncherTheme {
  return value === "light" || value === "dark" || value === "system";
}

export function resolveTheme(
  theme: LauncherTheme,
  systemDark: boolean,
): Exclude<LauncherTheme, "system"> {
  return theme === "system" ? (systemDark ? "dark" : "light") : theme;
}

export function initialTheme(): LauncherTheme {
  const value: unknown =
    "__LICHT_THEME__" in window ? window.__LICHT_THEME__ : undefined;
  return isLauncherTheme(value) ? value : "dark";
}
