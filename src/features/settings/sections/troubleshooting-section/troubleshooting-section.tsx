import { useMutation, useQuery } from "@tanstack/react-query";
import { CopyIcon, FolderOpen, MessageSquareWarning } from "lucide-react";
import { toast } from "sonner";
import { formatBuildDiagnostics } from "@/api/build-info";
import {
  getBuildDiagnostics,
  getLogsFolder,
  openFeedback,
  openLogsFolder,
} from "@/api/commands";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { friendlyError, messageOf } from "@/lib/errors";

import { troubleshootingSectionStyles as styles } from "./troubleshooting-section.styles";

function reportFailure(error: unknown) {
  toast.error(friendlyError(messageOf(error)));
}

export function TroubleshootingSection() {
  const logsFolder = useQuery({
    queryKey: ["logs-folder"],
    queryFn: getLogsFolder,
    staleTime: Number.POSITIVE_INFINITY,
  });
  const diagnostics = useQuery({
    queryKey: ["build-diagnostics"],
    queryFn: getBuildDiagnostics,
    staleTime: Number.POSITIVE_INFINITY,
  });
  const details = diagnostics.data
    ? formatBuildDiagnostics(diagnostics.data)
    : null;

  const openLogs = useMutation({
    mutationFn: openLogsFolder,
    onError: reportFailure,
  });
  const copy = useMutation({
    mutationFn: (text: string) => navigator.clipboard.writeText(text),
    onSuccess: () => toast.success("Build information copied."),
    onError: reportFailure,
  });
  const report = useMutation({
    mutationFn: async (text: string) => {
      await navigator.clipboard.writeText(text);
      await openFeedback();
    },
    onSuccess: () =>
      toast.success("Build information copied. Paste it into your report."),
    onError: reportFailure,
  });

  return (
    <div className={styles.root}>
      <Card title="Logs">
        <div className={styles.row}>
          <div className={styles.rowText}>
            <p className={styles.rowLabel}>Launcher logs</p>
            <p className={styles.rowDescription}>
              One file per day; the last 7 days are kept.
            </p>
            {logsFolder.data ? (
              <p className={styles.path} title={logsFolder.data}>
                {logsFolder.data}
              </p>
            ) : null}
            {logsFolder.error ? (
              <p className={styles.error}>
                Log folder unavailable: {messageOf(logsFolder.error)}
              </p>
            ) : null}
          </div>
          <Button
            type="button"
            variant="outline"
            className={styles.control}
            disabled={openLogs.isPending}
            onClick={() => openLogs.mutate()}
          >
            <FolderOpen data-icon="inline-start" aria-hidden />
            Open logs folder
          </Button>
        </div>
      </Card>
      <Card title="Build and system">
        <div className={styles.row}>
          <div className={styles.details}>
            {details?.split("\n").map((line) => (
              <p key={line}>{line}</p>
            ))}
            {diagnostics.error ? (
              <p className={styles.error}>
                Build information unavailable: {messageOf(diagnostics.error)}
              </p>
            ) : null}
          </div>
          <Button
            type="button"
            variant="outline"
            className={styles.control}
            disabled={details === null || copy.isPending}
            onClick={() => details && copy.mutate(details)}
          >
            <CopyIcon data-icon="inline-start" aria-hidden />
            Copy
          </Button>
        </div>
      </Card>
      <Card title="Report a problem">
        <div className={styles.row}>
          <div className={styles.rowText}>
            <p className={styles.rowDescription}>
              Opens the issue page on GitHub and copies the build information so
              you can paste it into your report. Attach the logs if you can.
            </p>
          </div>
          <Button
            type="button"
            variant="outline"
            className={styles.control}
            disabled={details === null || report.isPending}
            onClick={() => details && report.mutate(details)}
          >
            <MessageSquareWarning data-icon="inline-start" aria-hidden />
            Report a problem
          </Button>
        </div>
      </Card>
    </div>
  );
}
