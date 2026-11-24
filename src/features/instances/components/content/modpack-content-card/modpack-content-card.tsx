import { ClockIcon, EyeIcon, PackageIcon, TagIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { SOURCE_LABELS } from "@/features/discover/lib/discover-search";
import { useStoreModpackSummary } from "@/features/instances/hooks/use-modpack-summary";
import type {
  FileModpackOrigin,
  ModpackOrigin,
  StoreModpackOrigin,
} from "@/features/instances/lib/modpack-content";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";

import { modpackContentCardStyles as styles } from "./modpack-content-card.styles";

type CardProps = {
  entries: readonly InstanceContentEntry[];
  onView: () => void;
};

/**
 * The modpack an instance is linked to, with its version and release age. It
 * heads the content panel; what the pack ships is listed in the content dialog.
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
      title={summary.name}
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
      title={`Imported from a ${SOURCE_LABELS[origin.source]} file`}
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
  title: string;
  /** The installed pack version; `null` when unknown, left out for packs from a file. */
  version?: string | null;
  /** When that version was released; `null` when unknown, left out for packs from a file. */
  updated?: string | null;
}) {
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
        {version !== undefined || updated !== undefined ? (
          <dl className={styles.facts}>
            {version !== undefined ? (
              <div className={styles.versionFact}>
                <dt className={styles.label}>
                  <TagIcon aria-hidden />
                  Version
                </dt>
                <dd className={version ? styles.version : styles.unknown}>
                  {version ?? "Unknown"}
                </dd>
              </div>
            ) : null}
            {updated !== undefined ? (
              <div className={styles.fact}>
                <dt className={styles.label}>
                  <ClockIcon aria-hidden />
                  Updated
                </dt>
                <dd className={updated ? styles.value : styles.unknown}>
                  {updated ?? "Unknown"}
                </dd>
              </div>
            ) : null}
          </dl>
        ) : null}
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
