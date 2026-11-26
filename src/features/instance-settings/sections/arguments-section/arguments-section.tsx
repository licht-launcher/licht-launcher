import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import type { InstanceDraft } from "@/features/instance-settings/model/draft";

import { argumentsSectionStyles as styles } from "./arguments-section.styles";

export function ArgumentsSection({
  draft,
  globalHint,
  onChange,
}: {
  draft: InstanceDraft;
  /** What the instance uses while it does not override Java arguments. */
  globalHint: string;
  onChange: (patch: Partial<InstanceDraft>) => void;
}) {
  return (
    <>
      <div className={styles.row}>
        <div className={styles.rowText}>
          <label htmlFor="override-jvm" className={styles.rowLabel}>
            Customize Java arguments
          </label>
          <p id="override-jvm-hint" className={styles.rowDescription}>
            {draft.overrideJvmArguments
              ? "Set Java arguments separately for this instance."
              : globalHint}
          </p>
        </div>
        <Switch
          id="override-jvm"
          className={styles.control}
          aria-label="Customize Java arguments"
          aria-describedby="override-jvm-hint"
          checked={draft.overrideJvmArguments}
          onCheckedChange={(overrideJvmArguments) =>
            onChange({ overrideJvmArguments })
          }
        />
      </div>
      {draft.overrideJvmArguments ? (
        <div className={styles.field}>
          <label htmlFor="instance-jvm-arguments">Java arguments</label>
          <Input
            id="instance-jvm-arguments"
            placeholder="-XX:+UseG1GC"
            aria-describedby="instance-jvm-arguments-hint"
            value={draft.jvmArguments}
            onChange={(event) => onChange({ jvmArguments: event.target.value })}
          />
          <p id="instance-jvm-arguments-hint" className={styles.rowDescription}>
            Extra JVM flags, separated by spaces.
          </p>
        </div>
      ) : null}
    </>
  );
}
