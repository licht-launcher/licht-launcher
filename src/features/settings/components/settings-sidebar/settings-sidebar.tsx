import { useMutation, useQueryClient } from "@tanstack/react-query";
import { FolderOpen } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { buildIdentifier, formatBuildDiagnostics } from "@/api/build-info";
import { getBuildDiagnostics, openLogsFolder } from "@/api/commands";
import { Button } from "@/components/ui/button";
import { DialogTitle } from "@/components/ui/dialog";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { SECTIONS, type SectionId } from "@/features/settings/model/sections";
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
  const openLogs = useMutation({
    mutationFn: openLogsFolder,
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
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
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className={settingsSidebarStyles.navButton}
          disabled={openLogs.isPending}
          onClick={() => openLogs.mutate()}
        >
          <FolderOpen className={settingsSidebarStyles.navIcon} />
          <span className={settingsSidebarStyles.navLabel}>
            Open logs folder
          </span>
        </Button>
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
