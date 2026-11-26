import {
  BoxesIcon,
  CircleArrowUpIcon,
  CpuIcon,
  HardDriveDownloadIcon,
  SettingsIcon,
  SquareCodeIcon,
  WrenchIcon,
} from "lucide-react";

export const SECTIONS = [
  {
    id: "account",
    label: "General",
    description: "Accounts, appearance and launcher behavior.",
    icon: SettingsIcon,
  },
  {
    id: "instances",
    label: "Instances",
    description:
      "Applied to every instance unless that instance customizes the setting.",
    icon: BoxesIcon,
  },
  {
    id: "java",
    label: "Runtime",
    description: "Java paths and extra JVM arguments for launch.",
    icon: CpuIcon,
  },
  {
    id: "downloads",
    label: "Downloads",
    description: "Download speed and launcher storage.",
    icon: HardDriveDownloadIcon,
  },
  {
    id: "updater",
    label: "Updates",
    description: "How Licht Launcher checks and installs updates.",
    icon: CircleArrowUpIcon,
  },
  {
    id: "troubleshooting",
    label: "Troubleshooting",
    description: "Logs, build information and problem reports.",
    icon: WrenchIcon,
  },
  ...(import.meta.env.DEV
    ? [
        {
          id: "developers",
          label: "Developer Tools",
          description: "Development-only tools for testing the launcher.",
          icon: SquareCodeIcon,
        },
      ]
    : []),
] as const;

export type SectionId = (typeof SECTIONS)[number]["id"];

/** A known section id, or the first section for anything else. */
export function parseSectionId(value: unknown): SectionId {
  return SECTIONS.find((item) => item.id === value)?.id ?? "account";
}

export const JAVA_MAJORS = [
  { major: 25, field: "java25", label: "Java 25" },
  { major: 21, field: "java21", label: "Java 21" },
  { major: 17, field: "java17", label: "Java 17" },
  { major: 8, field: "java8", label: "Java 8" },
] as const;

export type JavaField = (typeof JAVA_MAJORS)[number]["field"];
export type JavaMajor = (typeof JAVA_MAJORS)[number];
