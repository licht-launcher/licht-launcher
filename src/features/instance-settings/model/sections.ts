import {
  InfoIcon,
  LayersIcon,
  PackageIcon,
  SlidersHorizontalIcon,
} from "lucide-react";

export const INSTANCE_SECTIONS = [
  {
    id: "general",
    label: "General",
    description: "Name, folder, and instance actions.",
    icon: InfoIcon,
  },
  {
    id: "base",
    label: "Instance base",
    description:
      "Platform and game version this instance launches with. Only Vanilla is available for now.",
    icon: LayersIcon,
  },
  {
    id: "modpack",
    label: "Modpack",
    description: "The modpack this instance was installed from.",
    icon: PackageIcon,
  },
  {
    id: "overrides",
    label: "Override Settings",
    description:
      "Turn on a category to use values from this instance instead of global Settings.",
    icon: SlidersHorizontalIcon,
  },
] as const;

export type InstanceSectionId = (typeof INSTANCE_SECTIONS)[number]["id"];

/** The Modpack section only applies to instances linked to a pack. */
export function visibleSections(linkedToModpack: boolean) {
  return INSTANCE_SECTIONS.filter(
    (section) => linkedToModpack || section.id !== "modpack",
  );
}

/** A section the instance no longer has, such as Modpack after unlinking, falls back to General. */
export function shownSection(
  section: InstanceSectionId,
  linkedToModpack: boolean,
): InstanceSectionId {
  return section === "modpack" && !linkedToModpack ? "general" : section;
}
