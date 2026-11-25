import { ClockIcon, EyeIcon, TagIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { InstanceIcon } from "@/features/instances/components/instance-icon";
import {
  fileModpackSummary,
  useStoreModpackSummary,
} from "@/features/instances/hooks/use-modpack-summary";
import type {
  FileModpackOrigin,
  ModpackOrigin,
  StoreModpackOrigin,
} from "@/features/instances/lib/modpack-content";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { LinkedModpack } from "@/lib/generated/LinkedModpack";
import { SOURCE_LABELS } from "@/lib/modpack-sources";

import { modpackContentCardStyles as styles } from "./modpack-content-card.styles";

type CardProps = {
  entries: readonly InstanceContentEntry[];
  /** The instance's own icon, cached on disk. */
  iconPath: string | null;
  onView: () => void;
};

/**
 * The modpack an instance is linked to, with its version and release age. It
 * heads the content panel; what the pack ships is listed in the content dialog.
 */
export function ModpackContentCard({
  origin,
  linked,
  ...props
}: CardProps & { origin: ModpackOrigin; linked: LinkedModpack | null }) {
  return origin.type === "modpack" ? (
    <StoreModpackCard origin={origin} linked={linked} {...props} />
  ) : (
    <FileModpackCard origin={origin} linked={linked} {...props} />
  );
}

function StoreModpackCard({
  origin,
  linked,
  ...props
}: CardProps & { origin: StoreModpackOrigin; linked: LinkedModpack | null }) {
  const summary = useStoreModpackSummary(origin, linked);
  return (
    <ModpackCard
      title={summary.name}
      version={summary.version}
      updated={summary.updated}
      {...props}
    />
  );
}

function FileModpackCard({
  origin,
  linked,
  ...props
}: CardProps & { origin: FileModpackOrigin; linked: LinkedModpack | null }) {
  // Older imports saved nothing about the pack, so only its source is known.
  if (linked === null) {
    return (
      <ModpackCard
        title={`Imported from a ${SOURCE_LABELS[origin.source]} file`}
        {...props}
      />
    );
  }
  const summary = fileModpackSummary(origin, linked);
  return (
    <ModpackCard title={summary.name} version={summary.version} {...props} />
  );
}

function ModpackCard({
  iconPath,
  title,
  version,
  updated,
  entries,
  onView,
}: CardProps & {
  title: string;
  /** The installed pack version; `null` when unknown, left out for packs from a file. */
  version?: string | null;
  /** When that version was released; `null` when unknown, left out for packs from a file. */
  updated?: string | null;
}) {
  return (
    <section aria-label="Modpack content" className={styles.root}>
      <InstanceIcon iconPath={iconPath} size="md" className={styles.icon} />
      <div className={styles.heading}>
        <h3 className={styles.title}>{title}</h3>
        {version !== undefined || updated !== undefined ? (
          <dl className={styles.facts}>
            {version !== undefined ? (
              <div className={styles.versionFact}>
                <dt className={styles.label}>Version</dt>
                <dd className={version ? styles.version : styles.unknown}>
                  <TagIcon aria-hidden />
                  <span className={styles.valueText}>
                    {version ?? "Unknown"}
                  </span>
                </dd>
              </div>
            ) : null}
            {updated !== undefined ? (
              <div className={styles.fact}>
                <dt className={styles.label}>Updated</dt>
                <dd className={updated ? styles.value : styles.unknown}>
                  <ClockIcon aria-hidden />
                  <span className={styles.valueText}>
                    {updated ?? "Unknown"}
                  </span>
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
