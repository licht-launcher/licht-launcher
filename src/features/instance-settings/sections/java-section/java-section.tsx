import { useMutation } from "@tanstack/react-query";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import type { InstanceDraft } from "@/features/instance-settings/model/draft";
import { browseJava } from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";

import { javaSectionStyles } from "./java-section.styles";

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
    <div className={javaSectionStyles.root}>
      <div className={javaSectionStyles.toggle}>
        <div className={javaSectionStyles.toggleRow}>
          <label htmlFor="override-java" className={javaSectionStyles.label}>
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
        <p id="override-java-hint" className={javaSectionStyles.hint}>
          Use a custom Java executable for this instance.
        </p>
      </div>
      {draft.overrideJava ? (
        <div className={javaSectionStyles.fields}>
          <label className={javaSectionStyles.field} htmlFor="instance-java">
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
            className={javaSectionStyles.action}
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
