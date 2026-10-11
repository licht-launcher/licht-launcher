import type { PlayPhase } from "@/features/instances/hooks/use-play-session";
import type { ConsoleLine } from "@/features/instances/model/console-line";
import { ConsoleSection } from "@/features/instances/sections/console-section";
import { PlaySection } from "@/features/instances/sections/play-section";
import type { DownloadProgress } from "@/lib/generated/DownloadProgress";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

export function HomePanel({
  instance,
  busy,
  playDisabled,
  phase,
  installing,
  progress,
  lines,
  onPlay,
  onStop,
  onConfigure,
}: {
  instance: InstanceEntry | null;
  busy: boolean;
  playDisabled: boolean;
  phase: PlayPhase;
  installing: boolean;
  progress: DownloadProgress | null;
  lines: ConsoleLine[];
  onPlay: () => void;
  onStop: () => void;
  onConfigure: () => void;
}) {
  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-muted/30 px-8 pt-8 pr-12 pb-8">
      <div className="shrink-0">
        <PlaySection
          instance={instance}
          busy={busy}
          playDisabled={playDisabled}
          phase={phase}
          installing={installing}
          progress={progress}
          onPlay={onPlay}
          onStop={onStop}
          onConfigure={onConfigure}
        />
      </div>
      <ConsoleSection lines={lines} />
    </div>
  );
}
