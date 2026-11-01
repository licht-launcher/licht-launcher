import { Stepper } from "@/components/ui/stepper";
import type { WizardStep } from "@/features/new-instance/model/wizard";

const STEP_LABELS: Record<WizardStep, string> = {
  source: "Source",
  platform: "Platform",
  details: "Details",
  modpack: "Modpack",
  "modpack-version": "Pack version",
  file: "Pack file",
  confirm: "Confirm",
};

export function WizardStepper({
  steps,
  currentIndex,
  onSelect,
}: {
  steps: WizardStep[];
  currentIndex: number;
  onSelect: (step: WizardStep) => void;
}) {
  return (
    <Stepper
      currentIndex={currentIndex}
      steps={steps.map((step) => ({ label: STEP_LABELS[step] }))}
      onSelect={(index) => {
        const step = steps[index];
        if (step !== undefined) {
          onSelect(step);
        }
      }}
    />
  );
}
