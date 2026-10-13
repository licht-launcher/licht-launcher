import {
  BoxesIcon,
  DownloadIcon,
  PaletteIcon,
  RefreshCwIcon,
  TerminalIcon,
  UserRoundIcon,
} from "lucide-react";

export const SECTIONS = [
  {
    id: "account",
    label: "Account",
    description: "Microsoft sign-in and offline player name.",
    icon: UserRoundIcon,
  },
  {
    id: "appearance",
    label: "Appearance",
    description: "How the launcher looks.",
    icon: PaletteIcon,
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
    label: "Java",
    description: "Java paths and extra JVM arguments for launch.",
    icon: TerminalIcon,
  },
  {
    id: "downloads",
    label: "Downloads",
    description: "How many files download at once.",
    icon: DownloadIcon,
  },
  {
    id: "updater",
    label: "Updater",
    description: "How Licht Launcher checks and installs updates.",
    icon: RefreshCwIcon,
  },
] as const;

export type SectionId = (typeof SECTIONS)[number]["id"];

export const JAVA_MAJORS = [
  { major: 25, field: "java25", label: "Java 25" },
  { major: 21, field: "java21", label: "Java 21" },
  { major: 17, field: "java17", label: "Java 17" },
  { major: 8, field: "java8", label: "Java 8" },
] as const;

export type JavaField = (typeof JAVA_MAJORS)[number]["field"];
export type JavaMajor = (typeof JAVA_MAJORS)[number];
