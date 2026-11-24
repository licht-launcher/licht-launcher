import {
  BlocksIcon,
  ClockIcon,
  EyeIcon,
  EyeOffIcon,
  ImageIcon,
  LockIcon,
  PackageIcon,
  SparklesIcon,
  TagIcon,
} from "lucide-react";
import { type ReactNode, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  useModpack,
  useModpackVersions,
} from "@/features/discover/hooks/use-modpacks";
import { formatTimeAgo } from "@/features/discover/lib/format";
import {
  countContent,
  type ModpackOrigin,
} from "@/features/instances/lib/modpack-content";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";

import { modpackContentCardStyles as styles } from "./modpack-content-card.styles";

const KIND_ICONS: Record<InstanceContentKind, ReactNode> = {
  mods: <BlocksIcon aria-hidden />,
  resourcePacks: <ImageIcon aria-hidden />,
  shaderPacks: <SparklesIcon aria-hidden />,
};

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
  return (
    <section aria-label="Modpack content" className={styles.root}>
      <div className={styles.main}>
        {origin.type === "modpack" && origin.iconUrl ? (
          <img src={origin.iconUrl} alt="" className={styles.icon} />
        ) : (
          <span aria-hidden className={styles.iconFallback}>
            <PackageIcon />
          </span>
        )}
        <div className={styles.heading}>
          <h3 className={styles.title}>Modpack content</h3>
          <p className={styles.counts}>
            {countContent(entries).map((count) => (
              <span key={count.kind} className={styles.count}>
                {KIND_ICONS[count.kind]}
                {count.label}
              </span>
            ))}
          </p>
        </div>
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
      </div>
      <div className={styles.details}>
        {origin.type === "modpack" ? (
          <StoreModpackDetails origin={origin} />
        ) : (
          <span className={styles.detail}>
            <LockIcon aria-hidden />
            Imported from a modpack file
          </span>
        )}
      </div>
    </section>
  );
}

function StoreModpackDetails({
  origin,
}: {
  origin: Extract<ModpackOrigin, { type: "modpack" }>;
}) {
  const [now] = useState(() => Date.now());
  const modpack = useModpack(origin.source, origin.projectId);
  const versions = useModpackVersions(origin.source, origin.projectId);
  if (modpack.isPending) {
    return <span className={styles.detail}>Loading modpack details…</span>;
  }
  if (modpack.isError) {
    return (
      <span className={styles.detail}>Modpack details are unavailable.</span>
    );
  }
  const version = versions.data?.find((item) => item.id === origin.versionId);
  const updated = formatTimeAgo(version?.publishedAt ?? null, now);
  const iconUrl = modpack.data.iconUrl ?? origin.iconUrl;
  return (
    <>
      <span className={styles.detail}>
        <LockIcon aria-hidden />
        Provided by
        {iconUrl ? (
          <img src={iconUrl} alt="" className={styles.providerIcon} />
        ) : null}
        <span className={styles.name}>{modpack.data.name}</span>
      </span>
      {version ? (
        <span className={styles.detail}>
          <TagIcon aria-hidden />
          {version.name}
        </span>
      ) : null}
      {updated ? (
        <span className={styles.detail}>
          <ClockIcon aria-hidden />
          Updated {updated}
        </span>
      ) : null}
    </>
  );
}
