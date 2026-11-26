import { useMutation } from "@tanstack/react-query";
import { toast } from "sonner";
import { browseJava } from "@/api/commands";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import type { InstanceDraft } from "@/features/instance-settings/model/draft";
import { friendlyError, messageOf } from "@/lib/errors";

import { javaSectionStyles as styles } from "./java-section.styles";

export function JavaSection({
  draft,
  globalHint,
  onChange,
}: {
  draft: InstanceDraft;
  /** What the instance uses while it does not override Java. */
  globalHint: string;
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
    <>
      <div className={styles.row}>
        <div className={styles.rowText}>
          <label htmlFor="override-java" className={styles.rowLabel}>
            Customize Java installation
          </label>
          <p id="override-java-hint" className={styles.rowDescription}>
            {draft.overrideJava
              ? "Use a custom Java executable for this instance."
              : globalHint}
          </p>
        </div>
        <Switch
          id="override-java"
          className={styles.control}
          aria-label="Customize Java installation"
          aria-describedby="override-java-hint"
          checked={draft.overrideJava}
          onCheckedChange={(overrideJava) => onChange({ overrideJava })}
        />
      </div>
      {draft.overrideJava ? (
        <div className={styles.field}>
          <label htmlFor="instance-java">Java path</label>
          <div className={styles.pathRow}>
            <Input
              id="instance-java"
              className={styles.pathInput}
              value={draft.javaPath}
              placeholder="C:\Program Files\Java\bin\java.exe"
              onChange={(event) => onChange({ javaPath: event.target.value })}
            />
            <Button
              type="button"
              variant="outline"
              className={styles.control}
              disabled={browse.isPending}
              onClick={() => {
                browse.mutate();
              }}
            >
              Browse
            </Button>
          </div>
        </div>
      ) : null}
    </>
  );
}
