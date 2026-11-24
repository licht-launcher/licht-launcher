import { EyeIcon, PackageIcon } from "lucide-react";
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
      title={`Provided by ${summary.name}`}
      version={summary.version}
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
      version={null}
      {...props}
    />
  );
}

function ModpackCard({
  iconUrl,
  title,
  version,
  entries,
  onView,
}: CardProps & {
  iconUrl: string | null;
  title: string;
  version: string | null;
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
        <div className={styles.titleLine}>
          <h3 className={styles.title}>{title}</h3>
          {version ? <span className={styles.version}>{version}</span> : null}
        </div>
        <dl className={styles.included}>
          <dt className={styles.label}>Included</dt>
          <dd className={styles.counts}>
            {counts.length === 0
              ? "No content"
              : counts.map((count) => (
                  <span key={count.kind} className={styles.count}>
                    <ContentKindIcon kind={count.kind} size="sm" tone="muted" />
                    {count.label}
                  </span>
                ))}
          </dd>
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
