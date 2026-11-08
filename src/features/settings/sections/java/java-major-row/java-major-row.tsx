import { useQuery } from "@tanstack/react-query";
import { DownloadIcon, FolderOpenIcon, SearchIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useDebounced } from "@/features/settings/hooks/use-debounced";
import type { JavaMajor } from "@/features/settings/model/sections";
import { JavaDetectDialog } from "@/features/settings/sections/java/java-detect-dialog";
import { JavaMark } from "@/features/settings/sections/java/java-mark";
import { mergeInstallations } from "@/features/settings/sections/java/merge-installations";
import { javaInstallationStatus } from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";

export function JavaMajorRow({
  row,
  value,
  found,
  runtimePath,
  checkReady,
  detectionsPending,
  busy,
  onChange,
  onInstall,
  onRefreshDetections,
  onBrowse,
}: {
  row: JavaMajor;
  value: string;
  found: string[];
  runtimePath: string | null;
  checkReady: boolean;
  detectionsPending: boolean;
  busy: boolean;
  onChange: (path: string) => void;
  onInstall: () => void;
  onRefreshDetections: () => void;
  onBrowse: () => void;
}) {
  const [open, setOpen] = useState(false);
  const trimmed = value.trim();
  const debounced = useDebounced(trimmed, 300);
  const status = useQuery({
    queryKey: ["java-installation-status", row.field, debounced],
    queryFn: () =>
      javaInstallationStatus({
        java25: null,
        java21: null,
        java17: null,
        java8: null,
        [row.field]: debounced,
      }),
    enabled: checkReady && trimmed !== "" && debounced === trimmed,
    staleTime: Number.POSITIVE_INFINITY,
    retry: false,
  });
  useEffect(() => {
    if (status.error) {
      toast.error(friendlyError(messageOf(status.error)));
    }
  }, [status.error]);
  const checking =
    trimmed !== "" &&
    (debounced !== trimmed || status.isFetching || !status.isFetched);
  const valid =
    !checking && debounced === trimmed && status.data?.[row.field] === true;
  const installations = mergeInstallations(found, runtimePath);

  return (
    <div className="flex flex-col gap-2">
      <label
        className="flex flex-col gap-0.5 text-sm leading-snug"
        htmlFor={row.field}
      >
        <span className="font-normal">{row.label}</span>
        <span className="flex items-center gap-2">
          <Input
            id={row.field}
            placeholder="path/to/java"
            value={value}
            disabled={busy}
            onChange={(event) => onChange(event.target.value)}
          />
          <JavaMark label={row.label} checking={checking} valid={valid} />
        </span>
      </label>
      <div className="flex flex-wrap gap-2">
        <Button
          type="button"
          variant="outline"
          size="default"
          aria-label={`Recommended install for ${row.label}`}
          disabled={busy || checking || valid}
          onClick={onInstall}
        >
          <DownloadIcon data-icon="inline-start" aria-hidden />
          Recommended install
        </Button>
        <Button
          type="button"
          variant="outline"
          size="default"
          aria-label={`Detect installations for ${row.label}`}
          disabled={
            installations.length === 0 || (detectionsPending && !runtimePath)
          }
          onClick={() => {
            onRefreshDetections();
            setOpen(true);
          }}
        >
          <SearchIcon data-icon="inline-start" aria-hidden />
          Detect installations
        </Button>
        <Button
          type="button"
          variant="outline"
          size="default"
          aria-label={`Browse for ${row.label}`}
          disabled={busy}
          onClick={onBrowse}
        >
          <FolderOpenIcon data-icon="inline-start" aria-hidden />
          Browse
        </Button>
      </div>
      <JavaDetectDialog
        open={open}
        onOpenChange={setOpen}
        row={row}
        value={value}
        installations={installations}
        onSelect={onChange}
      />
    </div>
  );
}
