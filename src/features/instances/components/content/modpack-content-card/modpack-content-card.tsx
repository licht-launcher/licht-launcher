import {
  ClockIcon,
  EyeIcon,
  FileArchiveIcon,
  LockIcon,
  PackageIcon,
} from "lucide-react";
import { type ReactNode, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  useModpack,
  useModpackVersions,
} from "@/features/discover/hooks/use-modpacks";
import { SOURCE_LABELS } from "@/features/discover/lib/discover-search";
import { formatTimeAgo } from "@/features/discover/lib/format";
import { ContentKindIcon } from "@/features/instances/components/content/content-kind-icon";
import {
  countContent,
  type ModpackOrigin,
} from "@/features/instances/lib/modpack-content";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";

import { modpackContentCardStyles as styles } from "./modpack-content-card.styles";

type StoreOrigin = Extract<ModpackOrigin, { type: "modpack" }>;

/**
 * Summary of the modpack an instance was created from and the content it ships.
 * It sits at the top of the content panel, above the content the user added.
 */
export function ModpackContentCard({
  origin,
  entries,
  onView,
}: {
  origin: ModpackOrigin;
  entries: readonly InstanceContentEntry[];
  onView: () => void;
}) {
  const included = <IncludedContent entries={entries} />;
  const action = (
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
  );
  return (
    <section aria-label="Modpack content" className={styles.root}>
      {origin.type === "modpack" ? (
        <StoreModpack origin={origin} included={included} action={action} />
      ) : (
        <CardHeader
          iconUrl={null}
          title="Imported modpack"
          subtitle={
            <span className={styles.subtitlePart}>
              <FileArchiveIcon aria-hidden />
              Installed from a {SOURCE_LABELS[origin.source]} file
            </span>
          }
          included={included}
          action={action}
        />
      )}
    </section>
  );
}

function StoreModpack({
  origin,
  included,
  action,
}: {
  origin: StoreOrigin;
  included: ReactNode;
  action: ReactNode;
}) {
  const [now] = useState(() => Date.now());
  const modpack = useModpack(origin.source, origin.projectId);
  const versions = useModpackVersions(origin.source, origin.projectId);
  const version = versions.data?.find((item) => item.id === origin.versionId);
  const updated = formatTimeAgo(version?.publishedAt ?? null, now);
  const sourceLabel = SOURCE_LABELS[origin.source];
  const provider = modpack.isPending
    ? "Loading modpack details…"
    : modpack.isError
      ? "Modpack details are unavailable."
      : modpack.data.author
        ? `by ${modpack.data.author} · ${sourceLabel}`
        : sourceLabel;
  return (
    <CardHeader
      iconUrl={modpack.data?.iconUrl ?? origin.iconUrl}
      title={modpack.data?.name ?? "Modpack"}
      version={version?.name ?? null}
      subtitle={
        <>
          <span className={styles.subtitlePart}>
            <LockIcon aria-hidden />
            {provider}
          </span>
          {updated && modpack.isSuccess ? (
            <span className={styles.subtitlePart}>
              <ClockIcon aria-hidden />
              Updated {updated}
            </span>
          ) : null}
        </>
      }
      included={included}
      action={action}
    />
  );
}

function CardHeader({
  iconUrl,
  title,
  version = null,
  subtitle,
  included,
  action,
}: {
  iconUrl: string | null;
  title: string;
  version?: string | null;
  subtitle: ReactNode;
  included: ReactNode;
  action: ReactNode;
}) {
  return (
    <div className={styles.header}>
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
        <p className={styles.subtitle}>{subtitle}</p>
        <p className={styles.counts}>{included}</p>
      </div>
      {action}
    </div>
  );
}

function IncludedContent({
  entries,
}: {
  entries: readonly InstanceContentEntry[];
}) {
  const counts = countContent(entries);
  if (counts.length === 0) return "No content";
  return counts.map((count) => (
    <span key={count.kind} className={styles.count}>
      <ContentKindIcon kind={count.kind} size="sm" />
      {count.label}
    </span>
  ));
}
