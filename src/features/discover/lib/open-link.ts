import { toast } from "sonner";

import { openExternalUrl } from "@/api/commands";
import { friendlyError, messageOf } from "@/lib/errors";

/** Opens a link in the system browser; a refused or failed open shows a toast. */
export function openLink(url: string) {
  openExternalUrl(url).catch((error: unknown) =>
    toast.error(friendlyError(messageOf(error))),
  );
}
