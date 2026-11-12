import {
  BookOpenIcon,
  BugIcon,
  CodeIcon,
  type LucideIcon,
  MessagesSquareIcon,
  UserIcon,
} from "lucide-react";
import { type ReactNode, useId } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { LoaderBadge } from "@/features/discover/components/loader-badge";
import { formatUpdatedAt } from "@/features/discover/lib/format";
import { openLink } from "@/features/discover/lib/open-link";
import type { ModpackDetails } from "@/lib/generated/ModpackDetails";
import type { ModpackLinks } from "@/lib/generated/ModpackLinks";

import { modpackSidebarStyles } from "./modpack-sidebar.styles";

const LINKS: readonly {
  key: keyof ModpackLinks;
  label: string;
  icon: LucideIcon;
}[] = [
  { key: "source", label: "Source code", icon: CodeIcon },
  { key: "issues", label: "Report issues", icon: BugIcon },
  { key: "wiki", label: "Wiki", icon: BookOpenIcon },
  { key: "discord", label: "Discord", icon: MessagesSquareIcon },
];

/** Compatibility, links, creators and details of a pack, beside its tabs. */
export function ModpackSidebar({ modpack }: { modpack: ModpackDetails }) {
  const links = LINKS.flatMap((link) => {
    const url = modpack.links[link.key];
    return url ? [{ ...link, url }] : [];
  });
  const published = formatUpdatedAt(modpack.publishedAt);
  const updated = formatUpdatedAt(modpack.updatedAt);
  const license = modpack.license;
  const licenseUrl = license?.url ?? null;

  return (
    <div className={modpackSidebarStyles.root}>
      {(modpack.gameVersions.length > 0 || modpack.loaders.length > 0) && (
        <Card title="Compatibility" className={modpackSidebarStyles.card}>
          {modpack.gameVersions.length > 0 && (
            <SidebarList label="Minecraft versions">
              {modpack.gameVersions.map((version) => (
                <li key={version}>
                  <Badge>{version}</Badge>
                </li>
              ))}
            </SidebarList>
          )}
          {modpack.loaders.length > 0 && (
            <SidebarList label="Loaders">
              {modpack.loaders.map((loader) => (
                <li key={loader}>
                  <LoaderBadge loader={loader} />
                </li>
              ))}
            </SidebarList>
          )}
        </Card>
      )}
      {links.length > 0 && (
        <Card title="Links" className={modpackSidebarStyles.card}>
          <ul className={modpackSidebarStyles.links}>
            {links.map((link) => (
              <li key={link.key}>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className={modpackSidebarStyles.link}
                  onClick={() => openLink(link.url)}
                >
                  <link.icon aria-hidden />
                  {link.label}
                </Button>
              </li>
            ))}
          </ul>
        </Card>
      )}
      {modpack.author && (
        <Card title="Creators" className={modpackSidebarStyles.card}>
          <p className={modpackSidebarStyles.creator}>
            <UserIcon aria-hidden />
            {modpack.author}
          </p>
        </Card>
      )}
      {(license || published || updated) && (
        <Card title="Details" className={modpackSidebarStyles.card}>
          <dl className={modpackSidebarStyles.details}>
            {license && (
              <div className={modpackSidebarStyles.detail}>
                <dt className={modpackSidebarStyles.term}>License</dt>
                <dd className={modpackSidebarStyles.value}>
                  {licenseUrl ? (
                    <button
                      type="button"
                      className={modpackSidebarStyles.inlineLink}
                      onClick={() => openLink(licenseUrl)}
                    >
                      {license.name}
                    </button>
                  ) : (
                    license.name
                  )}
                </dd>
              </div>
            )}
            {published && (
              <div className={modpackSidebarStyles.detail}>
                <dt className={modpackSidebarStyles.term}>Published</dt>
                <dd className={modpackSidebarStyles.value}>{published}</dd>
              </div>
            )}
            {updated && (
              <div className={modpackSidebarStyles.detail}>
                <dt className={modpackSidebarStyles.term}>Updated</dt>
                <dd className={modpackSidebarStyles.value}>{updated}</dd>
              </div>
            )}
          </dl>
        </Card>
      )}
    </div>
  );
}

function SidebarList({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  const labelId = useId();
  return (
    <div className={modpackSidebarStyles.group}>
      <p id={labelId} className={modpackSidebarStyles.term}>
        {label}
      </p>
      <ul aria-labelledby={labelId} className={modpackSidebarStyles.tags}>
        {children}
      </ul>
    </div>
  );
}
