import { ClockIcon, EyeIcon, TagIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { SOURCE_LABELS } from "@/features/discover/lib/discover-search";
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

/** One column of the card; `value` is `null` when it is not known. */
type CardFact = { kind: "version" | "updated"; value: string | null };

const FACTS = {
  version: { label: "Version", icon: <TagIcon aria-hidden /> },
  updated: { label: "Updated", icon: <ClockIcon aria-hidden /> },
} as const;

function StoreModpackCard({
  origin,
  linked,
  ...props
}: CardProps & { origin: StoreModpackOrigin; linked: LinkedModpack | null }) {
  const summary = useStoreModpackSummary(origin, linked);
  return (
    <ModpackCard
      title={summary.name}
      facts={[
        { kind: "version", value: summary.version },
        { kind: "updated", value: summary.updated },
      ]}
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
        facts={[]}
        {...props}
      />
    );
  }
  const summary = fileModpackSummary(origin, linked);
  // An archive carries no release date, so there is no Updated column.
  return (
    <ModpackCard
      title={summary.name}
      facts={[{ kind: "version", value: summary.version }]}
      {...props}
    />
  );
}

function ModpackCard({
  iconPath,
  title,
  facts,
  entries,
  onView,
}: CardProps & { title: string; facts: readonly CardFact[] }) {
  return (
    <section aria-label="Modpack content" className={styles.root}>
      <InstanceIcon iconPath={iconPath} size="md" className={styles.icon} />
      <div className={styles.heading}>
        <h3 className={styles.title}>{title}</h3>
        {facts.length > 0 ? (
          <dl className={styles.facts}>
            {facts.map((fact) => (
              <div
                key={fact.kind}
                className={
                  fact.kind === "version" ? styles.versionFact : styles.fact
                }
              >
                <dt className={styles.label}>{FACTS[fact.kind].label}</dt>
                <dd className={fact.value ? styles.value : styles.unknown}>
                  {FACTS[fact.kind].icon}
                  <span className={styles.valueText}>
                    {fact.value ?? "Unknown"}
                  </span>
                </dd>
              </div>
            ))}
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
