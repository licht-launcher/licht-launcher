import {
  BlocksIcon,
  EyeIcon,
  EyeOffIcon,
  FileArchiveIcon,
  ImageIcon,
  LockIcon,
  PackageIcon,
  SparklesIcon,
} from "lucide-react";
import { type ReactNode, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  useModpack,
  useModpackVersions,
} from "@/features/discover/hooks/use-modpacks";
import { SOURCE_LABELS } from "@/features/discover/lib/discover-search";
import { formatTimeAgo } from "@/features/discover/lib/format";
import {
  countContent,
  type ModpackOrigin,
} from "@/features/instances/lib/modpack-content";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";

import { modpackContentCardStyles as styles } from "./modpack-content-card.styles";

const KIND_ICONS: Record<InstanceContentKind, ReactNode> = {
  mods: <BlocksIcon aria-hidden className={styles.countIcon} />,
  resourcePacks: <ImageIcon aria-hidden className={styles.countIcon} />,
  shaderPacks: <SparklesIcon aria-hidden className={styles.countIcon} />,
};

type StoreOrigin = Extract<ModpackOrigin, { type: "modpack" }>;

/** Summary of the modpack an instance was created from and the content it ships. */
export function ModpackContentCard({
  origin,
  entries,
  viewing,
  onViewingChange,
}: {
  origin: ModpackOrigin;
  entries: readonly InstanceContentEntry[];
  viewing: boolean;
  onViewingChange: (viewing: boolean) => void;
}) {
  const included = <IncludedContent entries={entries} />;
  const action = (
    <Button
      type="button"
      variant="outline"
      aria-pressed={viewing}
      disabled={entries.length === 0}
      onClick={() => onViewingChange(!viewing)}
    >
      {viewing ? (
        <EyeOffIcon data-icon="inline-start" aria-hidden />
      ) : (
        <EyeIcon data-icon="inline-start" aria-hidden />
      )}
      {viewing ? "Hide content" : "View content"}
    </Button>
  );
  return (
    <section aria-label="Modpack content" className={styles.root}>
      {origin.type === "modpack" ? (
        <StoreModpack origin={origin} included={included} action={action} />
      ) : (
        <>
          <CardHeader
            iconUrl={null}
            title="Imported modpack"
            subtitle={
              <>
                <FileArchiveIcon aria-hidden />
                Installed from a {SOURCE_LABELS[origin.source]} file
              </>
            }
            action={action}
          />
          <dl className={styles.facts}>
            <Fact label="Included">{included}</Fact>
          </dl>
        </>
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
  const subtitle = modpack.isPending
    ? "Loading modpack details…"
    : modpack.isError
      ? "Modpack details are unavailable."
      : modpack.data.author
        ? `by ${modpack.data.author} · ${sourceLabel}`
        : sourceLabel;
  return (
    <>
      <CardHeader
        iconUrl={modpack.data?.iconUrl ?? origin.iconUrl}
        title={modpack.data?.name ?? "Modpack"}
        subtitle={
          <>
            <LockIcon aria-hidden />
            {subtitle}
          </>
        }
        action={action}
      />
      <dl className={styles.facts}>
        <Fact label="Version">
          {version ? (
            <Badge>{version.name}</Badge>
          ) : (
            <span className={styles.muted}>Unknown</span>
          )}
        </Fact>
        <Fact label="Updated">
          <span className={updated ? undefined : styles.muted}>
            {updated ?? "Unknown"}
          </span>
        </Fact>
        <Fact label="Included">{included}</Fact>
      </dl>
    </>
  );
}

function CardHeader({
  iconUrl,
  title,
  subtitle,
  action,
}: {
  iconUrl: string | null;
  title: string;
  subtitle: ReactNode;
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
        <h3 className={styles.title}>{title}</h3>
        <p className={styles.subtitle}>{subtitle}</p>
      </div>
      {action}
    </div>
  );
}

function Fact({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className={styles.fact}>
      <dt className={styles.factLabel}>{label}</dt>
      <dd className={styles.factValue}>{children}</dd>
    </div>
  );
}

function IncludedContent({
  entries,
}: {
  entries: readonly InstanceContentEntry[];
}) {
  const counts = countContent(entries);
  if (counts.length === 0)
    return <span className={styles.muted}>No content</span>;
  return counts.map((count) => (
    <span key={count.kind} className={styles.count}>
      {KIND_ICONS[count.kind]}
      {count.label}
    </span>
  ));
}
