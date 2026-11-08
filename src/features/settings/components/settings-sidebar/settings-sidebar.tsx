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

import { settingsSidebarStyles } from "./settings-sidebar.styles";

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
        className={settingsSidebarStyles.navButton}
        aria-current={section === item.id ? "page" : undefined}
        onClick={() => onSectionChange(item.id)}
      >
        <Icon className={settingsSidebarStyles.navIcon} />
        <span className={settingsSidebarStyles.navLabel}>{item.label}</span>
      </Button>
    );
  }

  return (
    <div className={settingsSidebarStyles.root}>
      <DialogTitle className={settingsSidebarStyles.title}>
        Settings
      </DialogTitle>
      <nav aria-label="Settings sections" className={settingsSidebarStyles.nav}>
        {SECTIONS.filter((item) => item.id !== "developers").map(renderSection)}
      </nav>
      <div className={settingsSidebarStyles.footer}>
        {SECTIONS.some((item) => item.id === "developers") ? (
          <nav
            aria-label="Developer settings"
            className={settingsSidebarStyles.footerLinks}
          >
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
                className={settingsSidebarStyles.versionButton}
                aria-label="Copy build and system information"
                disabled={copy.isPending}
                onClick={() => copy.mutate()}
              >
                <span className={settingsSidebarStyles.versionLabel}>
                  {identifier}
                </span>
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
