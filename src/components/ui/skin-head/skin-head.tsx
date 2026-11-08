import { convertFileSrc } from "@tauri-apps/api/core";
import { UserRoundIcon } from "lucide-react";
import { useState } from "react";

import { skinHeadStyles } from "./skin-head.styles";

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
      className={skinHeadStyles.root}
    >
      {source !== null ? (
        <img
          src={source}
          alt=""
          draggable={false}
          className={skinHeadStyles.image}
          onError={() => setFailedPath(headPath ?? null)}
        />
      ) : (
        <UserRoundIcon className={skinHeadStyles.fallback} />
      )}
    </span>
  );
}
