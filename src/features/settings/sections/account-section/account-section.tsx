import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Loader2Icon, LogInIcon } from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import {
  activeMicrosoftAccount,
  beginMicrosoftLogin,
  listMicrosoftAccounts,
  logoutMicrosoftAccount,
  setActiveMicrosoftAccount,
} from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import type { AccountProfile } from "@/lib/generated/AccountProfile";

export function AccountSection() {
  const queryClient = useQueryClient();
  const [loginError, setLoginError] = useState<string | null>(null);

  const accounts = useQuery({
    queryKey: ["microsoft-accounts"],
    queryFn: listMicrosoftAccounts,
    retry: false,
  });
  const active = useQuery({
    queryKey: ["microsoft-account-active"],
    queryFn: activeMicrosoftAccount,
    retry: false,
  });

  const begin = useMutation({
    mutationFn: beginMicrosoftLogin,
    onSuccess: async () => {
      setLoginError(null);
      await queryClient.invalidateQueries({
        queryKey: ["microsoft-accounts"],
      });
      await queryClient.invalidateQueries({
        queryKey: ["microsoft-account-active"],
      });
    },
    onError: (error) => {
      setLoginError(friendlyError(messageOf(error)));
    },
  });

  async function activate(profile: AccountProfile) {
    setLoginError(null);
    try {
      await setActiveMicrosoftAccount(profile.uuid);
      await queryClient.invalidateQueries({
        queryKey: ["microsoft-account-active"],
      });
    } catch (error) {
      setLoginError(friendlyError(messageOf(error)));
    }
  }

  async function signOut(profile: AccountProfile) {
    setLoginError(null);
    try {
      await logoutMicrosoftAccount(profile.uuid);
      await queryClient.invalidateQueries({ queryKey: ["microsoft-accounts"] });
      await queryClient.invalidateQueries({
        queryKey: ["microsoft-account-active"],
      });
    } catch (error) {
      setLoginError(friendlyError(messageOf(error)));
    }
  }

  const activeUuid = active.data?.uuid ?? null;
  const profiles = accounts.data ?? [];

  return (
    <Card title="Account">
      <div className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 flex-col gap-0.5">
          <h4 className="text-sm font-normal">Sign in with Microsoft</h4>
          <p className="text-sm text-muted-foreground">
            Sign in to launch with your purchased game.
          </p>
        </div>
        <Button
          type="button"
          size="default"
          disabled={begin.isPending}
          onClick={() => {
            begin.mutate();
          }}
        >
          {begin.isPending ? (
            <Loader2Icon
              data-icon="inline-start"
              aria-hidden
              className="animate-spin"
            />
          ) : (
            <LogInIcon data-icon="inline-start" aria-hidden />
          )}
          {begin.isPending ? "Waiting for browser…" : "Sign in"}
        </Button>
      </div>
      {begin.isPending ? (
        <p className="text-sm text-muted-foreground">
          Complete sign-in in your browser, then return here.
        </p>
      ) : null}
      {loginError !== null ? (
        <p className="text-sm text-destructive" role="alert">
          {loginError}
        </p>
      ) : null}
      {profiles.length > 0 ? (
        <ul className="flex flex-col gap-2">
          {profiles.map((profile) => {
            const isActive = profile.uuid === activeUuid;
            return (
              <li
                key={profile.uuid}
                className="flex items-center justify-between gap-2 rounded-md border border-border px-3 py-2"
              >
                <div className="min-w-0">
                  <p className="truncate text-sm font-normal">{profile.name}</p>
                  <p className="truncate font-mono text-xs text-muted-foreground">
                    {profile.uuid}
                  </p>
                </div>
                <div className="flex shrink-0 gap-2">
                  {isActive ? (
                    <span className="text-xs text-muted-foreground">
                      Active
                    </span>
                  ) : (
                    <Button
                      type="button"
                      size="default"
                      variant="outline"
                      onClick={() => {
                        void activate(profile);
                      }}
                    >
                      Use
                    </Button>
                  )}
                  <Button
                    type="button"
                    size="default"
                    variant="outline"
                    onClick={() => {
                      void signOut(profile);
                    }}
                  >
                    Sign out
                  </Button>
                </div>
              </li>
            );
          })}
        </ul>
      ) : null}
    </Card>
  );
}
