import { cn } from "cn";
import type * as React from "react";

import { inputStyles } from "./input.styles";

type InputProps = Omit<React.ComponentProps<"input">, "size"> & {
  startIcon?: React.ReactNode;
  size?: "sm" | "default";
};

function Input({
  className,
  type,
  startIcon,
  size = "default",
  ...props
}: InputProps) {
  const inputClassName = cn(
    inputStyles.input,
    startIcon != null ? inputStyles.inputWithIcon : null,
    startIcon == null ? className : null,
  );

  const input = (
    <input
      type={type}
      data-slot="input"
      data-size={size}
      className={inputClassName}
      {...props}
    />
  );

  if (startIcon == null) {
    return input;
  }

  return (
    <div className={cn(inputStyles.wrapper, className)}>
      <span className={inputStyles.startIcon}>{startIcon}</span>
      {input}
    </div>
  );
}

export { Input };
