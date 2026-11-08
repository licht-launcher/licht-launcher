import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import type { InstanceDraft } from "@/features/instance-settings/model/draft";

export function ArgumentsSection({
  draft,
  onChange,
}: {
  draft: InstanceDraft;
  onChange: (patch: Partial<InstanceDraft>) => void;
}) {
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-1">
        <div className="flex items-center justify-between gap-3">
          <label htmlFor="override-jvm" className="text-sm">
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
        <p id="override-jvm-hint" className="text-sm text-muted-foreground">
          Set Java arguments separately for this instance.
        </p>
      </div>
      {draft.overrideJvmArguments ? (
        <div className="flex flex-col gap-1">
          <label
            className="flex flex-col gap-1 text-sm"
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
          <p className="text-sm text-muted-foreground">
            Extra JVM flags, separated by spaces.
          </p>
        </div>
      ) : null}
    </div>
  );
}
