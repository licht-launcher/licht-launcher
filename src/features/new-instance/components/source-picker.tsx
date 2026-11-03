import {
  ChevronRightIcon,
  FileUpIcon,
  PackageIcon,
  WrenchIcon,
} from "lucide-react";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

const tones = {
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

type Tone = keyof typeof tones;

/** Highlights a keyword with the row's tone instead of a heavier weight. */
function Key({ tone, children }: { tone: Tone; children: ReactNode }) {
  return (
    <strong className={cn("font-medium", tones[tone].text)}>{children}</strong>
  );
}

function Code({ children }: { children: ReactNode }) {
  return (
    <code className="rounded bg-black/25 px-1 py-0.5 font-mono text-xs text-foreground">
      {children}
    </code>
  );
}

function SourceRow({
  tone,
  icon,
  title,
  description,
  disabled,
  soon = false,
  onSelect,
}: {
  tone: Tone;
  icon: ReactNode;
  title: string;
  description: ReactNode;
  disabled: boolean;
  soon?: boolean;
  onSelect: () => void;
}) {
  return (
    <li>
      <button
        type="button"
        disabled={disabled}
        onClick={onSelect}
        className={cn(
          "group flex w-full cursor-pointer items-center gap-4 rounded-lg border border-border/60 px-4 py-3 text-left outline-none",
          "transition-[background-color,border-color,transform] duration-200 ease-out",
          "hover:border-border hover:bg-muted/50 focus-visible:ring-3 focus-visible:ring-ring/50",
          "disabled:pointer-events-none disabled:opacity-60",
        )}
      >
        <span
          className={cn(
            "flex size-11 shrink-0 items-center justify-center rounded-lg transition-transform duration-200 ease-out motion-safe:group-hover:scale-105",
            tones[tone].well,
          )}
        >
          {icon}
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="text-sm font-medium tracking-tight">{title}</span>
          <span className="text-xs leading-snug text-muted-foreground">
            {description}
          </span>
        </span>
        {soon ? (
          <span className="shrink-0 rounded-full border border-warning/40 px-2 py-0.5 text-xs text-warning">
            Soon
          </span>
        ) : (
          <ChevronRightIcon
            aria-hidden
            className="size-5 shrink-0 text-muted-foreground transition-[transform,color] duration-200 ease-out group-hover:text-foreground motion-safe:group-hover:translate-x-0.5"
          />
        )}
      </button>
    </li>
  );
}

export function SourcePicker({
  busy,
  onScratch,
  onUpload,
}: {
  busy: boolean;
  onScratch: () => void;
  onUpload: () => void;
}) {
  return (
    <ul aria-label="Create from" className="flex flex-col gap-2">
      <SourceRow
        tone="cool"
        icon={<WrenchIcon aria-hidden className="size-5" strokeWidth={2} />}
        title="From scratch"
        description={
          <>
            Choose a <Key tone="cool">platform</Key>, a{" "}
            <Key tone="cool">name</Key> and a{" "}
            <Key tone="cool">game version</Key>.
          </>
        }
        disabled={busy}
        onSelect={onScratch}
      />
      <SourceRow
        tone="warning"
        icon={<PackageIcon aria-hidden className="size-5" strokeWidth={2} />}
        title="From a modpack"
        description={
          <>
            Browse popular packs from <Key tone="warning">Modrinth</Key> and{" "}
            <Key tone="warning">CurseForge</Key>.
          </>
        }
        disabled
        soon
        onSelect={() => {}}
      />
      <SourceRow
        tone="file"
        icon={<FileUpIcon aria-hidden className="size-5" strokeWidth={2} />}
        title="Upload a pack file"
        description={
          <>
            Use a local <Code>.mrpack</Code> or CurseForge <Code>.zip</Code>.
          </>
        }
        disabled={busy}
        onSelect={onUpload}
      />
    </ul>
  );
}
