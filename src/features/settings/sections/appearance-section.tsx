import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import type { Draft } from "@/features/settings/model/draft";

export function AppearanceSection({
  draft,
  onChange,
}: {
  draft: Draft;
  onChange: (patch: Partial<Draft>) => void;
}) {
  return (
    <div className="flex max-w-md flex-col gap-6">
      <div className="flex max-w-xs flex-col gap-1 text-sm">
        <span>Color theme</span>
        <Select
          value={draft.theme}
          onValueChange={(theme) => {
            if (theme === "dark") {
              onChange({ theme });
            }
          }}
        >
          <SelectTrigger aria-label="Color theme">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="dark">Dark</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 flex-col gap-1">
          <label
            htmlFor="disable-custom-title-bar"
            className="text-sm font-medium"
          >
            Disable custom title bar
          </label>
          <p
            id="disable-custom-title-bar-hint"
            className="text-sm text-muted-foreground"
          >
            Use the operating system title bar instead of Licht's. This change
            applies after restart.
          </p>
        </div>
        <Switch
          id="disable-custom-title-bar"
          className="shrink-0"
          aria-label="Disable custom title bar"
          aria-describedby="disable-custom-title-bar-hint"
          checked={draft.disableCustomTitleBar}
          onCheckedChange={(disableCustomTitleBar) =>
            onChange({ disableCustomTitleBar })
          }
        />
      </div>
    </div>
  );
}
