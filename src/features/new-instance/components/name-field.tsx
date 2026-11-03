import { TagIcon } from "lucide-react";

import { Input } from "@/components/ui/input";
import { FormSection } from "@/features/new-instance/components/form-section";

export function NameField({
  value,
  onChange,
}: {
  value: string;
  onChange: (name: string) => void;
}) {
  return (
    <FormSection
      divided={false}
      icon={<TagIcon aria-hidden className="size-4" strokeWidth={2} />}
      title="Name your instance"
      description="You can rename it later from the library."
    >
      <label
        className="flex flex-col gap-1.5 text-sm font-medium"
        htmlFor="instance-name"
      >
        <Input
          id="instance-name"
          aria-label="Instance Name"
          value={value}
          autoComplete="off"
          placeholder="Survival"
          onChange={(event) => {
            onChange(event.target.value);
          }}
        />
      </label>
    </FormSection>
  );
}
