import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { PlusIcon } from "lucide-react";
import { useRef, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { SkinHead } from "@/components/ui/skin-head";
import { useTitleBarDecorations } from "@/hooks/use-title-bar-decorations";
import {
  activateOfflineAccount,
  listMicrosoftAccounts,
  setActiveMicrosoftAccount,
} from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import type { AccountProfile } from "@/lib/generated/AccountProfile";

export function PlayerAccountButton({
  onAddAccount,
}: {
  onAddAccount: () => void;
}) {
  const queryClient = useQueryClient();
  const { username, offlineUsername, activeUuid, headPath } =
    useTitleBarDecorations();
  const [open, setOpen] = useState(false);
  const openingSettings = useRef(false);
  const accounts = useQuery({
    queryKey: ["microsoft-accounts"],
    queryFn: listMicrosoftAccounts,
    enabled: open,
    retry: false,
  });

  async function accountChanged(profile: AccountProfile | null) {
    setOpen(false);
    await queryClient.cancelQueries({ queryKey: ["microsoft-account-active"] });
    queryClient.setQueryData(["microsoft-account-active"], profile);
    await queryClient.invalidateQueries({ queryKey: ["microsoft-accounts"] });
  }

  const select = useMutation({
    mutationFn: setActiveMicrosoftAccount,
    onSuccess: accountChanged,
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
  });
  const offline = useMutation({
    mutationFn: activateOfflineAccount,
    onSuccess: () => accountChanged(null),
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
  });
  const busy = select.isPending || offline.isPending;

  return (
    <DropdownMenu open={open} onOpenChange={setOpen}>
      <DropdownMenuTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-7 max-w-48 justify-start gap-2 rounded-md px-2 text-xs text-muted-foreground data-[state=open]:border-foreground/20 data-[state=open]:bg-muted data-[state=open]:text-foreground"
          aria-label={`Account: ${username}`}
        >
          <SkinHead key={activeUuid || username} headPath={headPath} />
          <span className="truncate">{username}</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        side="bottom"
        align="end"
        aria-label="Player accounts"
        className="min-w-56 font-brand"
        onCloseAutoFocus={(event) => {
          if (openingSettings.current) {
            event.preventDefault();
            openingSettings.current = false;
          }
        }}
      >
        {accounts.isPending ? (
          <p
            role="status"
            className="px-2 py-1.5 text-xs text-muted-foreground"
          >
            Loading accounts…
          </p>
        ) : null}
        {accounts.isError ? (
          <>
            <p role="alert" className="px-2 py-1.5 text-xs text-destructive">
              Could not load accounts.
            </p>
            <DropdownMenuItem
              onSelect={(event) => {
                event.preventDefault();
                void accounts.refetch();
              }}
            >
              Retry
            </DropdownMenuItem>
          </>
        ) : null}
        {accounts.isSuccess && accounts.data.length === 0 ? (
          <p className="px-2 py-1.5 text-xs text-muted-foreground">
            No Microsoft accounts added
          </p>
        ) : null}
        <DropdownMenuRadioGroup
          value={activeUuid || "offline"}
          onValueChange={(uuid) => {
            if (busy || uuid === (activeUuid || "offline")) return;
            if (uuid === "offline") offline.mutate();
            else select.mutate(uuid);
          }}
        >
          {(accounts.data ?? []).map((profile) => (
            <DropdownMenuRadioItem
              key={profile.uuid}
              value={profile.uuid}
              disabled={busy}
            >
              {profile.name}
            </DropdownMenuRadioItem>
          ))}
          <DropdownMenuSeparator />
          <DropdownMenuRadioItem value="offline" disabled={busy}>
            Offline account · {offlineUsername}
          </DropdownMenuRadioItem>
        </DropdownMenuRadioGroup>
        <DropdownMenuSeparator />
        <DropdownMenuItem
          disabled={busy}
          onSelect={() => {
            openingSettings.current = true;
            onAddAccount();
          }}
        >
          <PlusIcon aria-hidden />
          Add account
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
