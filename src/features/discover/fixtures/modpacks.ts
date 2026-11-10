import type { ModpackDetails } from "@/lib/generated/ModpackDetails";
import type { ModpackListItem } from "@/lib/generated/ModpackListItem";
import type { ModpackSearchPage } from "@/lib/generated/ModpackSearchPage";
import type { ModpackVersionItem } from "@/lib/generated/ModpackVersionItem";

export function modpackListItem(
  overrides: Partial<ModpackListItem> = {},
): ModpackListItem {
  return {
    id: "1KVo5zza",
    source: "modrinth",
    name: "Fabulously Optimized",
    description: "Beautiful graphics and speedy performance.",
    downloads: 19_192_625,
    iconUrl: "https://cdn.modrinth.com/data/1KVo5zza/icon.webp",
    author: "robotkoer",
    updatedAt: "2026-11-01T10:43:19.109719Z",
    loaders: ["fabric"],
    categories: ["lightweight", "multiplayer", "optimization", "quests"],
    ...overrides,
  };
}

export function modpackSearchPage(
  overrides: Partial<ModpackSearchPage> = {},
): ModpackSearchPage {
  return {
    items: [
      modpackListItem(),
      modpackListItem({
        id: "381671",
        source: "curseforge",
        name: "All the Mods 9",
        description: "A kitchen-sink pack.",
        downloads: 8_400_000,
        iconUrl: null,
        author: "ATMTeam",
      }),
    ],
    page: 1,
    pageCount: 3,
    unavailableSources: [],
    ...overrides,
  };
}

export function modpackDetails(
  overrides: Partial<ModpackDetails> = {},
): ModpackDetails {
  return {
    id: "1KVo5zza",
    source: "modrinth",
    name: "Fabulously Optimized",
    summary: "Beautiful graphics and speedy performance.",
    author: "robotkoer",
    downloads: 19_192_625,
    iconUrl: "https://cdn.modrinth.com/data/1KVo5zza/icon.webp",
    categories: ["optimization", "lightweight"],
    updatedAt: "2026-11-01T10:43:19.109719Z",
    gallery: ["https://cdn.modrinth.com/data/1KVo5zza/images/title.webp"],
    projectUrl: "https://modrinth.com/modpack/fabulously-optimized",
    descriptionHtml:
      '<h2>Features</h2><p>Read the <a href="https://download.fo/install" rel="noopener noreferrer">install guide</a>.</p>',
    ...overrides,
  };
}

export function modpackVersions(): ModpackVersionItem[] {
  return [
    {
      id: "v7",
      name: "7.0.0",
      gameVersions: ["1.21.10"],
      loaders: ["fabric"],
    },
    {
      id: "v6",
      name: "6.4.0",
      gameVersions: ["1.21.8"],
      loaders: ["fabric"],
    },
  ];
}
