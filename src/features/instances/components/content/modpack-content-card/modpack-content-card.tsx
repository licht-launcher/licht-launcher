import { EyeIcon, LockIcon, PackageIcon } from "lucide-react";
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
 * Which pack provides an instance's content and how much it ships. It heads the
 * content panel; the pack's author and release age live in the content dialog.
 */
export function ModpackContentCard({
  origin,
  ...props
}: CardProps & { origin: ModpackOrigin }) {
  return origin.type === "modpack" ? (
    <StoreModpackCard origin={origin} {...props} />
  ) : (
    <FileModpackCard origin={origin} {...props} />
  );
}

function StoreModpackCard({
  origin,
  ...props
}: CardProps & { origin: StoreModpackOrigin }) {
  const summary = useStoreModpackSummary(origin);
  return (
    <ModpackCard
      iconUrl={summary.iconUrl}
      title={
        <>
          <LockIcon aria-hidden className={styles.lock} />
          <span className={styles.titleName}>{summary.name}</span>
        </>
      }
      version={summary.version}
      updated={summary.updated}
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
      title={
        <span className={styles.titleName}>
          Imported from a {SOURCE_LABELS[origin.source]} file
        </span>
      }
      {...props}
    />
  );
}

function ModpackCard({
  iconUrl,
  title,
  version,
  updated,
  entries,
  onView,
}: CardProps & {
  iconUrl: string | null;
  title: ReactNode;
  /** The installed pack version; `null` when unknown, left out for packs from a file. */
  version?: string | null;
  /** When that version was released; `null` when unknown, left out for packs from a file. */
  updated?: string | null;
}) {
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
        <h3 className={styles.title}>{title}</h3>
        <dl className={styles.facts}>
          {version !== undefined ? (
            <div className={styles.versionFact}>
              <dt className={styles.label}>Version</dt>
              <dd className={version ? styles.version : styles.unknown}>
                {version ?? "Unknown"}
              </dd>
            </div>
          ) : null}
          {updated !== undefined ? (
            <div className={styles.fact}>
              <dt className={styles.label}>Updated</dt>
              <dd className={updated ? styles.value : styles.unknown}>
                {updated ?? "Unknown"}
              </dd>
            </div>
          ) : null}
          <div className={styles.includedFact}>
            <dt className={styles.label}>Included</dt>
            <dd className={styles.counts}>
              {counts.length === 0
                ? "No content"
                : counts.map((count) => (
                    <span key={count.kind} className={styles.count}>
                      <ContentKindIcon
                        kind={count.kind}
                        size="sm"
                        tone="muted"
                      />
                      {count.label}
                    </span>
                  ))}
            </dd>
          </div>
        </dl>
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
