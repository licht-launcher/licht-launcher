import { toast } from "sonner";
import { openSettingsWindow } from "@/api/commands";
import type { SectionId } from "@/features/settings/model/sections";
import { friendlyError, messageOf } from "@/lib/errors";

export function requestOpenSettings(section: SectionId = "account") {
  openSettingsWindow(section).catch((error: unknown) => {
    toast.error(friendlyError(messageOf(error)));
  });
}
