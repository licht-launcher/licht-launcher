import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { DialogTitle } from "@/components/ui/dialog";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { SECTIONS, type SectionId } from "@/features/settings/model/sections";
import { buildIdentifier, formatBuildDiagnostics } from "@/lib/build-info";
import { getBuildDiagnostics } from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";

export function SettingsSidebar({
  section,
  onSectionChange,
}: {
  section: SectionId;
  onSectionChange: (section: SectionId) => void;
}) {
  const identifier = buildIdentifier();
  const queryClient = useQueryClient();
  const [copyTooltipOpen, setCopyTooltipOpen] = useState(false);
  const copy = useMutation({
    mutationFn: async () => {
      const diagnostics = await queryClient.fetchQuery({
        queryKey: ["build-diagnostics"],
        queryFn: getBuildDiagnostics,
        staleTime: Number.POSITIVE_INFINITY,
        retry: false,
      });
      await navigator.clipboard.writeText(formatBuildDiagnostics(diagnostics));
    },
    onSettled: () => setCopyTooltipOpen(true),
  });

  function renderSection(item: (typeof SECTIONS)[number]) {
    const Icon = item.icon;
    return (
      <Button
        key={item.id}
        type="button"
        variant={section === item.id ? "secondary" : "ghost"}
        size="sm"
        className="h-auto flex items-center justify-start gap-2 px-2.5 py-1.75 font-medium"
        aria-current={section === item.id ? "page" : undefined}
        onClick={() => onSectionChange(item.id)}
      >
        <Icon className="size-4 shrink-0" />
        <span className="min-w-0 truncate">{item.label}</span>
      </Button>
    );
  }

  return (
    <div className="flex w-48 shrink-0 flex-col gap-4 border-r border-border px-3 py-4">
      <DialogTitle className="sr-only">Settings</DialogTitle>
      <nav aria-label="Settings sections" className="flex flex-col gap-0">
        {SECTIONS.filter((item) => item.id !== "developers").map(renderSection)}
      </nav>
      <div className="mt-auto flex min-w-0 flex-col gap-2">
        {SECTIONS.some((item) => item.id === "developers") ? (
          <nav aria-label="Developer settings" className="flex flex-col">
            {SECTIONS.filter((item) => item.id === "developers").map(
              renderSection,
            )}
          </nav>
        ) : null}
        <TooltipProvider>
          <Tooltip open={copyTooltipOpen} onOpenChange={setCopyTooltipOpen}>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="xs"
                className="h-auto w-full justify-start px-2.5 py-1 font-mono text-xs font-normal text-muted-foreground transition-none hover:bg-transparent hover:text-muted-foreground dark:hover:bg-transparent"
                aria-label="Copy build and system information"
                disabled={copy.isPending}
                onClick={() => copy.mutate()}
              >
                <span className="truncate">{identifier}</span>
              </Button>
            </TooltipTrigger>
            <TooltipContent side="right">
              {copy.error
                ? `Could not copy: ${friendlyError(messageOf(copy.error))}`
                : copy.isSuccess
                  ? "Copied!"
                  : "Copy build and system information"}
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </div>
    </div>
  );
}
