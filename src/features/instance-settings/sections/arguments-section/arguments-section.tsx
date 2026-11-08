import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import type { InstanceDraft } from "@/features/instance-settings/model/draft";

import { argumentsSectionStyles } from "./arguments-section.styles";

export function ArgumentsSection({
  draft,
  onChange,
}: {
  draft: InstanceDraft;
  onChange: (patch: Partial<InstanceDraft>) => void;
}) {
  return (
    <div className={argumentsSectionStyles.root}>
      <div className={argumentsSectionStyles.toggle}>
        <div className={argumentsSectionStyles.toggleRow}>
          <label
            htmlFor="override-jvm"
            className={argumentsSectionStyles.label}
          >
            Customize Java arguments
          </label>
          <Switch
            id="override-jvm"
            aria-label="Customize Java arguments"
            aria-describedby="override-jvm-hint"
            checked={draft.overrideJvmArguments}
            onCheckedChange={(overrideJvmArguments) =>
              onChange({ overrideJvmArguments })
            }
          />
        </div>
        <p id="override-jvm-hint" className={argumentsSectionStyles.hint}>
          Set Java arguments separately for this instance.
        </p>
      </div>
      {draft.overrideJvmArguments ? (
        <div className={argumentsSectionStyles.toggle}>
          <label
            className={argumentsSectionStyles.field}
            htmlFor="instance-jvm-arguments"
          >
            Java arguments
            <Input
              id="instance-jvm-arguments"
              placeholder="-XX:+UseG1GC"
              value={draft.jvmArguments}
              onChange={(event) =>
                onChange({ jvmArguments: event.target.value })
              }
            />
          </label>
          <p className={argumentsSectionStyles.hint}>
            Extra JVM flags, separated by spaces.
          </p>
        </div>
      ) : null}
    </div>
  );
}
