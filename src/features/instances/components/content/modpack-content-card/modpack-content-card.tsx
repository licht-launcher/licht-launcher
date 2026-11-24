import { EyeIcon, FileArchiveIcon, LockIcon, PackageIcon } from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { SOURCE_LABELS } from "@/features/discover/lib/discover-search";
import { ContentKindIcon } from "@/features/instances/components/content/content-kind-icon";
import { useStoreModpackSummary } from "@/features/instances/hooks/use-modpack-summary";
import {
  countContent,
  type FileModpackOrigin,
  type ModpackOrigin,
  type StoreModpackOrigin,
} from "@/features/instances/lib/modpack-content";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";

import { modpackContentCardStyles as styles } from "./modpack-content-card.styles";

type CardProps = {
  entries: readonly InstanceContentEntry[];
  onView: () => void;
};

/**
 * The content an instance's modpack ships and which pack provides it. It heads
 * the content panel; the pack's author and release age live in the content dialog.
 */
export function ModpackContentCard({
  origin,
  onOpenModpack,
  ...props
}: CardProps & {
  origin: ModpackOrigin;
  /** Opens the pack's store page; without it the pack name is plain text. */
  onOpenModpack?: () => void;
}) {
  return origin.type === "modpack" ? (
    <StoreModpackCard
      origin={origin}
      onOpenModpack={onOpenModpack}
      {...props}
    />
  ) : (
    <FileModpackCard origin={origin} {...props} />
  );
}

function StoreModpackCard({
  origin,
  onOpenModpack,
  ...props
}: CardProps & { origin: StoreModpackOrigin; onOpenModpack?: () => void }) {
  const summary = useStoreModpackSummary(origin);
  return (
    <ModpackCard
      iconUrl={summary.iconUrl}
      provider={
        <>
          <LockIcon aria-hidden />
          Provided by
          {onOpenModpack ? (
            <Button
              type="button"
              variant="link"
              className={styles.providerLink}
              onClick={onOpenModpack}
            >
              {summary.name}
            </Button>
          ) : (
            <span className={styles.providerName}>{summary.name}</span>
          )}
          {summary.version ? (
            <span className={styles.version}>· {summary.version}</span>
          ) : null}
        </>
      }
      {...props}
    />
  );
}

function FileModpackCard({
  origin,
  ...props
}: CardProps & { origin: FileModpackOrigin }) {
  return (
    <ModpackCard
      iconUrl={null}
      provider={
        <>
          <FileArchiveIcon aria-hidden />
          Imported from a {SOURCE_LABELS[origin.source]} file
        </>
      }
      {...props}
    />
  );
}

function ModpackCard({
  iconUrl,
  provider,
  entries,
  onView,
}: CardProps & { iconUrl: string | null; provider: ReactNode }) {
  const counts = countContent(entries);
  return (
    <section aria-label="Modpack content" className={styles.root}>
      {iconUrl ? (
        <img src={iconUrl} alt="" className={styles.icon} />
      ) : (
        <span aria-hidden className={styles.iconFallback}>
          <PackageIcon />
        </span>
      )}
      <div className={styles.heading}>
        <h3 className={styles.title}>Modpack content</h3>
        <p className={styles.counts}>
          {counts.length === 0
            ? "No content"
            : counts.map((count) => (
                <span key={count.kind} className={styles.count}>
                  <ContentKindIcon kind={count.kind} size="sm" tone="muted" />
                  {count.label}
                </span>
              ))}
        </p>
        <p className={styles.provider}>{provider}</p>
      </div>
      <Button
        type="button"
        variant="outline"
        aria-haspopup="dialog"
        disabled={entries.length === 0}
        onClick={onView}
      >
        <EyeIcon data-icon="inline-start" aria-hidden />
        View content
      </Button>
    </section>
  );
}
