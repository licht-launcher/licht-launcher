import {
  BoxIcon,
  CheckCircle2Icon,
  FolderPlusIcon,
  type LucideIcon,
  PackageIcon,
  TagIcon,
} from "lucide-react";

import { Stepper } from "@/components/ui/stepper";
import type { WizardStep } from "@/features/new-instance/model/wizard";

const STEP_ICONS: Record<WizardStep, LucideIcon> = {
  source: FolderPlusIcon,
  platform: BoxIcon,
  details: TagIcon,
  modpack: PackageIcon,
  "modpack-version": TagIcon,
  confirm: CheckCircle2Icon,
};

const STEP_LABELS: Record<WizardStep, string> = {
  source: "Source",
  platform: "Platform",
  details: "Details",
  modpack: "Modpack",
  "modpack-version": "Pack version",
  confirm: "Confirm",
};

export function WizardStepper({
  steps,
  currentIndex,
}: {
  steps: WizardStep[];
  currentIndex: number;
}) {
  return (
    <Stepper
      currentIndex={currentIndex}
      steps={steps.map((step) => ({
        label: STEP_LABELS[step],
        icon: STEP_ICONS[step],
      }))}
    />
  );
}
