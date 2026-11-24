import { useMutation, useQuery } from "@tanstack/react-query";
import {
  CopyIcon,
  ExternalLinkIcon,
  FolderOpen,
  MessageSquareWarning,
} from "lucide-react";
import { Fragment } from "react";
import { toast } from "sonner";
import {
  buildDiagnosticsEntries,
  formatBuildDiagnostics,
} from "@/api/build-info";
import {
  getBuildDiagnostics,
  getLogsFolder,
  openFeedback,
  openLogsFolder,
} from "@/api/commands";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import type { Draft } from "@/features/settings/model/draft";
import { friendlyError, messageOf } from "@/lib/errors";

import { troubleshootingSectionStyles as styles } from "./troubleshooting-section.styles";

function reportFailure(error: unknown) {
  toast.error(friendlyError(messageOf(error)));
}

export function TroubleshootingSection({
  draft,
  onChange,
}: {
  draft: Draft;
  onChange: (patch: Partial<Draft>) => void;
}) {
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
  const entries = diagnostics.data
    ? buildDiagnosticsEntries(diagnostics.data)
    : null;
  const details = diagnostics.data
    ? formatBuildDiagnostics(diagnostics.data)
    : "";

  const openLogs = useMutation({
    mutationFn: openLogsFolder,
    onError: reportFailure,
  });
  const copy = useMutation({
    mutationFn: () => navigator.clipboard.writeText(details),
    onSuccess: () => toast.success("Build information copied."),
    onError: reportFailure,
  });
  const report = useMutation({
    mutationFn: openFeedback,
    onError: reportFailure,
  });

  return (
    <div className={styles.root}>
      <Card title="Report a problem">
        <div className={styles.row}>
          <div className={styles.rowText}>
            <p className={styles.rowLabel}>Open an issue on GitHub</p>
            <p className={styles.rowDescription}>
              Include the build information below and attach the logs if you
              can.
            </p>
          </div>
          <Button
            type="button"
            variant="outline"
            className={styles.control}
            disabled={report.isPending}
            onClick={() => report.mutate()}
          >
            <MessageSquareWarning data-icon="inline-start" aria-hidden />
            Report a problem
            <ExternalLinkIcon data-icon="inline-end" aria-hidden />
          </Button>
        </div>
      </Card>
      <Card title="Logs">
        <div className={styles.row}>
          <div className={styles.rowText}>
            <p className={styles.rowLabel}>Launcher logs</p>
            <p className={styles.rowDescription}>
              One file per launch; the 30 most recent are kept.
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
        <div className={styles.row}>
          <div className={styles.rowText}>
            <label htmlFor="verboseLogging" className={styles.rowLabel}>
              Verbose logging
            </label>
            <p id="verboseLogging-hint" className={styles.rowDescription}>
              Records extra detail to help diagnose problems. Log files grow
              faster.
            </p>
          </div>
          <Switch
            id="verboseLogging"
            className={styles.control}
            aria-describedby="verboseLogging-hint"
            checked={draft.verboseLogging}
            onCheckedChange={(checked) => onChange({ verboseLogging: checked })}
          />
        </div>
      </Card>
      <Card title="Build and system">
        <div className={styles.rowStart}>
          {entries ? (
            <dl className={styles.details}>
              {entries.map(({ label, value }) => (
                <Fragment key={label}>
                  <dt className={styles.term}>{label}</dt>
                  <dd className={styles.definition} title={value}>
                    {value}
                  </dd>
                </Fragment>
              ))}
            </dl>
          ) : (
            <p
              className={
                diagnostics.error ? styles.error : styles.rowDescription
              }
            >
              {diagnostics.error
                ? `Build information unavailable: ${messageOf(diagnostics.error)}`
                : "Loading build information…"}
            </p>
          )}
          <Button
            type="button"
            variant="outline"
            className={styles.control}
            disabled={entries === null || copy.isPending}
            onClick={() => copy.mutate()}
          >
            <CopyIcon data-icon="inline-start" aria-hidden />
            Copy
          </Button>
        </div>
      </Card>
    </div>
  );
}
