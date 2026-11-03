import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

/** Accent classes shared by a source row: the icon well and its keywords. */
export const tones = {
  cool: {
    well: "bg-accent-cool/20 text-accent-cool",
    text: "text-accent-cool",
  },
  warning: { well: "bg-warning/20 text-warning", text: "text-warning" },
  file: {
    well: "bg-accent-file/20 text-accent-file",
    text: "text-accent-file",
  },
} as const;

export type Tone = keyof typeof tones;

/** Highlights a keyword with the row's tone instead of a heavier weight. */
export function Key({ tone, children }: { tone: Tone; children: ReactNode }) {
  return (
    <strong className={cn("font-medium", tones[tone].text)}>{children}</strong>
  );
}

export function Code({ children }: { children: ReactNode }) {
  return (
    <code className="rounded bg-black/25 px-1 py-0.5 font-mono text-xs text-foreground">
      {children}
    </code>
  );
}
