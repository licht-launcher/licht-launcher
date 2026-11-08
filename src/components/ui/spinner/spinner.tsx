import { cn } from "cn";
import { Loader2Icon } from "lucide-react";
import type * as React from "react";

import { spinnerStyles } from "./spinner.styles";

function Spinner({ className, ...props }: React.ComponentProps<"svg">) {
  return (
    <Loader2Icon
      role="status"
      aria-label="Loading"
      className={cn(spinnerStyles.root, className)}
      {...props}
    />
  );
}

export { Spinner };
