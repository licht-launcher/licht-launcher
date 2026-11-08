import { useId } from "react";
import type { LauncherTheme } from "@/lib/generated/LauncherTheme";

import { themePreviewStyles } from "./theme-preview.styles";

const labels: Record<LauncherTheme, string> = {
  light: "Light theme preview",
  dark: "Dark theme preview",
  system: "System theme preview",
};

function WindowPalette({ theme }: { theme: Exclude<LauncherTheme, "system"> }) {
  return (
    <g data-theme-preview={theme}>
      <rect width="180" height="110" className={themePreviewStyles.window} />
      <rect width="180" height="16" className={themePreviewStyles.titleBar} />
      <circle
        cx="9"
        cy="8"
        r="2"
        className={themePreviewStyles.windowControl}
      />
      <circle
        cx="17"
        cy="8"
        r="2"
        className={themePreviewStyles.windowControl}
      />
      <circle
        cx="25"
        cy="8"
        r="2"
        className={themePreviewStyles.windowControl}
      />
      <rect
        y="16"
        width="44"
        height="94"
        className={themePreviewStyles.sidebar}
      />
      <rect
        x="8"
        y="28"
        width="28"
        height="5"
        rx="2"
        className={themePreviewStyles.sidebarActive}
      />
      <rect
        x="8"
        y="41"
        width="22"
        height="4"
        rx="2"
        className={themePreviewStyles.sidebarItem}
      />
      <rect
        x="8"
        y="53"
        width="26"
        height="4"
        rx="2"
        className={themePreviewStyles.sidebarItem}
      />
      <rect
        x="8"
        y="65"
        width="19"
        height="4"
        rx="2"
        className={themePreviewStyles.sidebarItem}
      />
      <rect
        x="56"
        y="28"
        width="68"
        height="6"
        rx="2"
        className={themePreviewStyles.heading}
      />
      <rect
        x="56"
        y="43"
        width="110"
        height="30"
        rx="4"
        className={themePreviewStyles.panel}
      />
      <rect
        x="56"
        y="82"
        width="48"
        height="12"
        rx="3"
        className={themePreviewStyles.primaryAction}
      />
      <rect
        x="112"
        y="82"
        width="54"
        height="12"
        rx="3"
        className={themePreviewStyles.secondaryAction}
      />
      <path d="M0 16H180M44 16V110" className={themePreviewStyles.divider} />
    </g>
  );
}

export function ThemePreview({ theme }: { theme: LauncherTheme }) {
  const id = useId();
  return (
    <svg
      role="img"
      aria-label={labels[theme]}
      viewBox="0 0 180 110"
      className={themePreviewStyles.root}
    >
      <title>{labels[theme]}</title>
      {theme === "system" ? (
        <>
          <defs>
            <clipPath id={`${id}-light`}>
              <rect width="90" height="110" />
            </clipPath>
            <clipPath id={`${id}-dark`}>
              <rect x="90" width="90" height="110" />
            </clipPath>
          </defs>
          <g clipPath={`url(#${id}-light)`}>
            <WindowPalette theme="light" />
          </g>
          <g clipPath={`url(#${id}-dark)`}>
            <WindowPalette theme="dark" />
          </g>
        </>
      ) : (
        <WindowPalette theme={theme} />
      )}
    </svg>
  );
}
