import { TagIcon } from "lucide-react";

import { Input } from "@/components/ui/input";
import { FormSection } from "@/features/new-instance/components/form-section";

export function NameField({
  value,
  placeholder,
  error,
  onChange,
}: {
  value: string;
  placeholder: string;
  /** Shown under the field, which is then marked invalid. */
  error?: string;
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
          aria-invalid={error !== undefined}
          aria-describedby={
            error === undefined ? undefined : "instance-name-error"
          }
          value={value}
          autoComplete="off"
          placeholder={placeholder}
          onChange={(event) => {
            onChange(event.target.value);
          }}
        />
      </label>
      {error === undefined ? null : (
        <p id="instance-name-error" className="text-xs text-destructive">
          {error}
        </p>
      )}
    </FormSection>
  );
}
