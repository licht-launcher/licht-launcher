import { useEffect, useState } from "react";
import { Input } from "@/components/ui/input";

import { memoryInputStyles } from "./memory-input.styles";

export function MemoryInput({
  valueMb,
  minimumMb,
  maximumMb,
  onChange,
}: {
  valueMb: number;
  minimumMb: number;
  maximumMb: number;
  onChange: (value: number) => void;
}) {
  const [text, setText] = useState(String(valueMb));
  useEffect(() => {
    setText(String(valueMb));
  }, [valueMb]);
  return (
    <div className={memoryInputStyles.root}>
      <Input
        aria-label="Memory Allocation (MB)"
        className={memoryInputStyles.input}
        type="number"
        min={minimumMb}
        max={maximumMb}
        step={1}
        value={text}
        onChange={(event) => {
          const next = event.target.value;
          setText(next);
          const value = Number(next);
          if (
            next !== "" &&
            Number.isInteger(value) &&
            value >= minimumMb &&
            value <= maximumMb
          )
            onChange(value);
        }}
        onBlur={() => {
          const parsed = Number(text);
          const value =
            text.trim() === "" || !Number.isFinite(parsed)
              ? valueMb
              : Math.min(maximumMb, Math.max(minimumMb, Math.round(parsed)));
          setText(String(value));
          if (value !== valueMb) onChange(value);
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            event.currentTarget.blur();
          }
        }}
      />
      <span className={memoryInputStyles.unit}>MB</span>
    </div>
  );
}
