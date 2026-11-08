import { convertFileSrc } from "@tauri-apps/api/core";
import { UserRoundIcon } from "lucide-react";
import { useState } from "react";

export function SkinHead({ headPath }: { headPath?: string }) {
  const [failedPath, setFailedPath] = useState<string | null>(null);
  let source: string | null = null;
  if (headPath !== undefined && headPath !== failedPath) {
    try {
      source = convertFileSrc(headPath);
    } catch {
      source = null;
    }
  }

  return (
    <span
      aria-hidden="true"
      data-slot="skin-head"
      className="relative inline-flex size-4 shrink-0 items-center justify-center overflow-hidden rounded-xs bg-muted"
    >
      {source !== null ? (
        <img
          src={source}
          alt=""
          draggable={false}
          className="size-full object-cover"
          onError={() => setFailedPath(headPath ?? null)}
        />
      ) : (
        <UserRoundIcon className="size-3.5 text-muted-foreground" />
      )}
    </span>
  );
}
