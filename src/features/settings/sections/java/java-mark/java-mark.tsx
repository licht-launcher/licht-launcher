import { CircleCheckIcon, CircleXIcon, LoaderCircleIcon } from "lucide-react";

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
        className="size-4 shrink-0 animate-spin text-muted-foreground"
        aria-label={`Checking ${label}`}
      />
    );
  }
  const Icon = valid ? CircleCheckIcon : CircleXIcon;
  return (
    <Icon
      role="img"
      className={
        valid
          ? "size-4 shrink-0 text-primary"
          : "size-4 shrink-0 text-destructive"
      }
      aria-label={valid ? `${label} is valid` : `${label} is invalid`}
    />
  );
}
