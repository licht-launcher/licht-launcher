import { CircleCheckIcon, CircleXIcon, LoaderCircleIcon } from "lucide-react";

import { javaMarkStyles } from "./java-mark.styles";

export function JavaMark({
  label,
  checking,
  valid,
}: {
  label: string;
  checking: boolean;
  valid: boolean;
}) {
  if (checking) {
    return (
      <LoaderCircleIcon
        role="img"
        className={javaMarkStyles.pending}
        aria-label={`Checking ${label}`}
      />
    );
  }
  const Icon = valid ? CircleCheckIcon : CircleXIcon;
  return (
    <Icon
      role="img"
      className={valid ? javaMarkStyles.found : javaMarkStyles.missing}
      aria-label={valid ? `${label} is valid` : `${label} is invalid`}
    />
  );
}
