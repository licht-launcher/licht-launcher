import { InfoIcon, LayersIcon, SlidersHorizontalIcon } from "lucide-react";

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
    id: "overrides",
    label: "Override Settings",
    description:
      "Turn on a category to use values from this instance instead of global Settings.",
    icon: SlidersHorizontalIcon,
  },
] as const;

export type InstanceSectionId = (typeof INSTANCE_SECTIONS)[number]["id"];
