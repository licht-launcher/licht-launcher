import { FileUpIcon, PackageIcon, WrenchIcon } from "lucide-react";

import { Code, Key } from "@/features/new-instance/components/highlight";
import { SourceRow } from "@/features/new-instance/components/source-row";

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
