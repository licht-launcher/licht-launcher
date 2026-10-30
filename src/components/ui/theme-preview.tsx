import { useId } from "react";
import type { LauncherTheme } from "@/lib/generated/LauncherTheme";

const labels: Record<LauncherTheme, string> = {
  light: "Light theme preview",
  dark: "Dark theme preview",
  system: "System theme preview",
};

function WindowPalette({ theme }: { theme: Exclude<LauncherTheme, "system"> }) {
  return (
    <g data-theme-preview={theme}>
      <rect width="180" height="110" className="fill-background" />
      <rect width="180" height="16" className="fill-muted" />
      <circle cx="9" cy="8" r="2" className="fill-muted-foreground" />
      <circle cx="17" cy="8" r="2" className="fill-muted-foreground" />
      <circle cx="25" cy="8" r="2" className="fill-muted-foreground" />
      <rect y="16" width="44" height="94" className="fill-secondary" />
      <rect
        x="8"
        y="28"
        width="28"
        height="5"
        rx="2"
        className="fill-primary"
      />
      <rect
        x="8"
        y="41"
        width="22"
        height="4"
        rx="2"
        className="fill-muted-foreground/40"
      />
      <rect
        x="8"
        y="53"
        width="26"
        height="4"
        rx="2"
        className="fill-muted-foreground/40"
      />
      <rect
        x="8"
        y="65"
        width="19"
        height="4"
        rx="2"
        className="fill-muted-foreground/40"
      />
      <rect
        x="56"
        y="28"
        width="68"
        height="6"
        rx="2"
        className="fill-foreground/70"
      />
      <rect
        x="56"
        y="43"
        width="110"
        height="30"
        rx="4"
        className="fill-muted"
      />
      <rect
        x="56"
        y="82"
        width="48"
        height="12"
        rx="3"
        className="fill-primary"
      />
      <rect
        x="112"
        y="82"
        width="54"
        height="12"
        rx="3"
        className="fill-secondary"
      />
      <path d="M0 16H180M44 16V110" className="fill-none stroke-border" />
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
      className="block h-auto w-full overflow-hidden rounded-md border border-border"
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
