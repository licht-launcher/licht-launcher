import { useMutation } from "@tanstack/react-query";
import { FolderOpenIcon } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Slider } from "@/components/ui/slider";
import type { Draft } from "@/features/settings/model/draft";
import { browseApplicationDirectory } from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";

export function DownloadsSection({
  draft,
  onChange,
}: {
  draft: Draft;
  onChange: (patch: Partial<Draft>) => void;
}) {
  const browseDirectory = useMutation({
    mutationFn: browseApplicationDirectory,
    onSuccess: (path) => {
      if (path) {
        onChange({ dataDirectory: path });
      }
    },
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  return (
    <div className="flex flex-col gap-6">
      <section className="flex flex-col gap-4">
        <div className="flex flex-col gap-1">
          <div className="flex items-baseline justify-between gap-3">
            <h3 id="download-concurrency-label" className="text-sm font-medium">
              Maximum simultaneous downloads
            </h3>
            <span className="text-sm text-muted-foreground">
              {draft.downloadConcurrency}
            </span>
          </div>
          <p className="text-sm text-muted-foreground">
            How many files download over the network at once.
          </p>
        </div>
        <Slider
          aria-labelledby="download-concurrency-label"
          min={1}
          max={16}
          step={1}
          value={[draft.downloadConcurrency]}
          onValueChange={(value) => {
            const next = value[0];
            if (next !== undefined) {
              onChange({ downloadConcurrency: next });
            }
          }}
        />
      </section>

      <section className="flex flex-col gap-4 border-t border-border pt-6">
        <div className="flex flex-col gap-1">
          <div className="flex items-baseline justify-between gap-3">
            <h3 id="write-concurrency-label" className="text-sm font-medium">
              Maximum simultaneous writes
            </h3>
            <span className="text-sm text-muted-foreground">
              {draft.writeConcurrency}
            </span>
          </div>
          <p className="text-sm text-muted-foreground">
            How many files are written to disk at once.
          </p>
        </div>
        <Slider
          aria-labelledby="write-concurrency-label"
          min={1}
          max={16}
          step={1}
          value={[draft.writeConcurrency]}
          onValueChange={(value) => {
            const next = value[0];
            if (next !== undefined) {
              onChange({ writeConcurrency: next });
            }
          }}
        />
      </section>

      <section className="flex flex-col gap-4 border-t border-border pt-6">
        <div className="flex flex-col gap-1">
          <h3 className="text-sm font-medium">Application directory</h3>
          <p className="text-sm text-muted-foreground">
            Where game data and launcher files are stored. This change applies
            after restart.
          </p>
        </div>
        <div className="flex gap-2">
          <Input
            id="application-directory"
            aria-label="Application directory"
            className="min-w-0 flex-1"
            value={draft.dataDirectory}
            onChange={(event) =>
              onChange({ dataDirectory: event.target.value })
            }
          />
          <Button
            type="button"
            variant="secondary"
            aria-label="Browse application directory"
            disabled={browseDirectory.isPending}
            onClick={() => {
              browseDirectory.mutate(draft.dataDirectory);
            }}
          >
            <FolderOpenIcon data-icon="inline-start" aria-hidden />
            Browse
          </Button>
        </div>
      </section>
    </div>
  );
}
