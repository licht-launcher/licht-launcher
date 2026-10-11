import { Input } from "@/components/ui/input";
import type { Draft } from "@/features/settings/model/draft";

export function AccountSection({
  draft,
  onChange,
}: {
  draft: Draft;
  onChange: (patch: Partial<Draft>) => void;
}) {
  return (
    <div className="flex max-w-sm flex-col gap-1">
      <label htmlFor="offline-username" className="text-sm font-medium">
        Username
      </label>
      <p id="offline-username-hint" className="text-sm text-muted-foreground">
        Offline name used when launching the game.
      </p>
      <Input
        id="offline-username"
        value={draft.username}
        autoComplete="nickname"
        aria-describedby="offline-username-hint"
        onChange={(event) => {
          onChange({ username: event.target.value });
        }}
      />
    </div>
  );
}
