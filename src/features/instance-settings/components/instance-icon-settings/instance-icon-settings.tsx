import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ImageIcon, PencilIcon, RotateCcwIcon } from "lucide-react";
import { toast } from "sonner";

import {
  browseInstanceIcon,
  resetInstanceIcon,
  setInstanceIcon,
} from "@/api/commands";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { InstanceIcon } from "@/features/instances/components/instance-icon";
import { friendlyError, messageOf } from "@/lib/errors";

import { instanceIconSettingsStyles as styles } from "./instance-icon-settings.styles";

/** The instance's icon, with actions to pick another image or go back to the default. */
export function InstanceIconSettings({
  folder,
  iconPath,
  customIcon,
  busy,
}: {
  folder: string;
  iconPath: string | null;
  /** The icon is one the user picked rather than the pack's or the default. */
  customIcon: boolean;
  /** Another action on the instance is running. */
  busy: boolean;
}) {
  const queryClient = useQueryClient();
  const change = useMutation({
    mutationFn: async () => {
      const path = await browseInstanceIcon();
      return path === null ? null : setInstanceIcon(folder, path);
    },
    onSuccess: async (entry) => {
      if (entry === null) return;
      await queryClient.invalidateQueries({ queryKey: ["instances"] });
      toast.success("Icon changed.");
    },
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
  });
  const reset = useMutation({
    mutationFn: () => resetInstanceIcon(folder),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["instances"] });
      toast.success("Icon reset.");
    },
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
  });
  const disabled = busy || change.isPending || reset.isPending;

  return (
    <div className={styles.row}>
      <div className={styles.rowText}>
        <p className={styles.rowLabel}>Icon</p>
        <p className={styles.rowDescription}>
          A PNG, JPEG, WebP or GIF image up to 2 MB.
        </p>
      </div>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button
            type="button"
            className={styles.trigger}
            disabled={disabled}
            aria-label="Instance icon"
          >
            <InstanceIcon iconPath={iconPath} size="lg" />
            <span className={styles.overlay}>
              <PencilIcon className={styles.overlayIcon} aria-hidden />
            </span>
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" aria-label="Instance icon">
          <DropdownMenuItem onSelect={() => change.mutate()}>
            <ImageIcon aria-hidden />
            Change icon
          </DropdownMenuItem>
          <DropdownMenuItem
            disabled={!customIcon}
            onSelect={() => reset.mutate()}
          >
            <RotateCcwIcon aria-hidden />
            Reset icon
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
