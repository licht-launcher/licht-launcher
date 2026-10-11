import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { DialogTitle } from "@/components/ui/dialog";
import { GitHubIcon } from "@/features/settings/components/github-icon";
import { SECTIONS, type SectionId } from "@/features/settings/model/sections";
import { appReleaseLabel } from "@/lib/app-release";
import { openRepository } from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";

export function SettingsSidebar({
  section,
  onSectionChange,
}: {
  section: SectionId;
  onSectionChange: (section: SectionId) => void;
}) {
  return (
    <div className="flex w-56 shrink-0 flex-col gap-4 bg-black/20 px-2 py-6">
      <DialogTitle className="border border-transparent pl-4 text-lg">
        Settings
      </DialogTitle>
      <nav aria-label="Settings sections" className="flex flex-col gap-1">
        {SECTIONS.map((item) => {
          const Icon = item.icon;
          return (
            <Button
              key={item.id}
              type="button"
              variant={section === item.id ? "secondary" : "ghost"}
              className="justify-start px-4"
              aria-current={section === item.id ? "page" : undefined}
              onClick={() => onSectionChange(item.id)}
            >
              <Icon />
              {item.label}
            </Button>
          );
        })}
      </nav>
      <div className="mt-auto flex items-center gap-2 border border-transparent px-4">
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <p className="text-sm">Licht Launcher</p>
          <p className="text-xs text-muted-foreground">{appReleaseLabel}</p>
        </div>
        <Button
          type="button"
          variant="ghost"
          size="icon-xs"
          className="shrink-0 text-muted-foreground"
          aria-label="GitHub repository"
          onClick={() => {
            void openRepository().catch((error: unknown) => {
              toast.error(friendlyError(messageOf(error)));
            });
          }}
        >
          <GitHubIcon />
        </Button>
      </div>
    </div>
  );
}
