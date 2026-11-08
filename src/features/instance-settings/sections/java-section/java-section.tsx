import { useMutation } from "@tanstack/react-query";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import type { InstanceDraft } from "@/features/instance-settings/model/draft";
import { browseJava } from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";

export function JavaSection({
  draft,
  onChange,
}: {
  draft: InstanceDraft;
  onChange: (patch: Partial<InstanceDraft>) => void;
}) {
  const browse = useMutation({
    mutationFn: () => browseJava(draft.javaPath),
    onSuccess: (path) => {
      if (path !== null) {
        onChange({ javaPath: path });
      }
    },
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-1">
        <div className="flex items-center justify-between gap-3">
          <label htmlFor="override-java" className="text-sm">
            Customize Java installation
          </label>
          <Switch
            id="override-java"
            aria-label="Customize Java installation"
            aria-describedby="override-java-hint"
            checked={draft.overrideJava}
            onCheckedChange={(overrideJava) => onChange({ overrideJava })}
          />
        </div>
        <p id="override-java-hint" className="text-sm text-muted-foreground">
          Use a custom Java executable for this instance.
        </p>
      </div>
      {draft.overrideJava ? (
        <div className="flex flex-col gap-2">
          <label
            className="flex flex-col gap-1 text-sm"
            htmlFor="instance-java"
          >
            Java path
            <Input
              id="instance-java"
              value={draft.javaPath}
              placeholder="C:\\Program Files\\Java\\bin\\java.exe"
              onChange={(event) => onChange({ javaPath: event.target.value })}
            />
          </label>
          <Button
            type="button"
            variant="secondary"
            size="sm"
            className="self-start"
            disabled={browse.isPending}
            onClick={() => {
              browse.mutate();
            }}
          >
            Browse
          </Button>
        </div>
      ) : null}
    </div>
  );
}
