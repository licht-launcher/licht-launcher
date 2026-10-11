import { cn } from "cn";
import type * as React from "react";

type InputProps = React.ComponentProps<"input"> & {
  startIcon?: React.ReactNode;
};

function Input({ className, type, startIcon, ...props }: InputProps) {
  const inputClassName = cn(
    "h-8 w-full min-w-0 rounded-lg border border-input bg-transparent px-2.5 py-1 text-base transition-colors outline-none select-text file:inline-flex file:h-6 file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:cursor-not-allowed disabled:bg-input/50 disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20 md:text-sm dark:bg-input/30 dark:disabled:bg-input/80 dark:aria-invalid:border-destructive/50 dark:aria-invalid:ring-destructive/40",
    startIcon != null ? "pl-8" : null,
    startIcon == null ? className : null,
  );

  const input = (
    <input
      type={type}
      data-slot="input"
      className={inputClassName}
      {...props}
    />
  );

  if (startIcon == null) {
    return input;
  }

  return (
    <div className={cn("relative min-w-0", className)}>
      <span className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground [&_svg]:size-4">
        {startIcon}
      </span>
      {input}
    </div>
  );
}

export { Input };
