import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { Draft } from "@/features/settings/model/draft";
import {
  activeMicrosoftAccount,
  beginMicrosoftLogin,
  listMicrosoftAccounts,
  logoutMicrosoftAccount,
  pollMicrosoftLogin,
  setActiveMicrosoftAccount,
} from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import type { AccountProfile } from "@/lib/generated/AccountProfile";
import type { DeviceCodeChallenge } from "@/lib/generated/DeviceCodeChallenge";

export function AccountSection({
  draft,
  onChange,
}: {
  draft: Draft;
  onChange: (patch: Partial<Draft>) => void;
}) {
  const queryClient = useQueryClient();
  const [challenge, setChallenge] = useState<DeviceCodeChallenge | null>(null);
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
    onSuccess: (next) => {
      setLoginError(null);
      setChallenge(next);
    },
    onError: (error) => {
      setChallenge(null);
      setLoginError(friendlyError(messageOf(error)));
    },
  });

  useEffect(() => {
    if (challenge === null) {
      return;
    }
    let cancelled = false;
    const started = Date.now();
    const tick = async () => {
      while (!cancelled) {
        const elapsed = (Date.now() - started) / 1000;
        if (elapsed > challenge.expiresInSeconds) {
          setLoginError(friendlyError("Microsoft device code expired"));
          setChallenge(null);
          return;
        }
        try {
          const profile = await pollMicrosoftLogin(challenge.deviceCode);
          if (cancelled) {
            return;
          }
          if (profile !== null) {
            setChallenge(null);
            await queryClient.invalidateQueries({
              queryKey: ["microsoft-accounts"],
            });
            await queryClient.invalidateQueries({
              queryKey: ["microsoft-account-active"],
            });
            return;
          }
        } catch (error) {
          if (cancelled) {
            return;
          }
          setChallenge(null);
          setLoginError(friendlyError(messageOf(error)));
          return;
        }
        await new Promise((resolve) => {
          window.setTimeout(resolve, challenge.intervalSeconds * 1000);
        });
      }
    };
    void tick();
    return () => {
      cancelled = true;
    };
  }, [challenge, queryClient]);

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
  const showOffline = activeUuid === null;

  return (
    <div className="flex max-w-lg flex-col gap-6">
      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between gap-3">
          <div>
            <p className="text-sm font-medium">Microsoft account</p>
            <p className="text-sm text-muted-foreground">
              Sign in to launch with your purchased game. Offline still works
              without a Microsoft account.
            </p>
          </div>
          <Button
            type="button"
            size="sm"
            disabled={begin.isPending || challenge !== null}
            onClick={() => {
              begin.mutate();
            }}
          >
            Sign in
          </Button>
        </div>
        {challenge !== null ? (
          <div className="rounded-md border border-border bg-muted/40 p-3 text-sm">
            <p>
              Open{" "}
              <a
                href={challenge.verificationUri}
                target="_blank"
                rel="noreferrer"
                className="underline"
              >
                {challenge.verificationUri}
              </a>{" "}
              and enter this code:
            </p>
            <p className="mt-2 font-mono text-lg tracking-widest">
              {challenge.userCode}
            </p>
          </div>
        ) : null}
        {loginError !== null ? (
          <p className="text-sm text-destructive" role="alert">
            {loginError}
          </p>
        ) : null}
        <ul className="flex flex-col gap-2">
          {(accounts.data ?? []).map((profile) => {
            const isActive = profile.uuid === activeUuid;
            return (
              <li
                key={profile.uuid}
                className="flex items-center justify-between gap-2 rounded-md border border-border px-3 py-2"
              >
                <div className="min-w-0">
                  <p className="truncate text-sm font-medium">{profile.name}</p>
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
                      size="sm"
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
                    size="sm"
                    variant="ghost"
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
      </div>

      {showOffline ? (
        <div className="flex max-w-sm flex-col gap-1">
          <label htmlFor="offline-username" className="text-sm font-medium">
            Offline username
          </label>
          <p
            id="offline-username-hint"
            className="text-sm text-muted-foreground"
          >
            Used when no Microsoft account is active.
          </p>
          <Input
            id="offline-username"
            value={draft.username}
            autoComplete="nickname"
            aria-describedby="offline-username-hint"
            onChange={(event) => {
              onChange({ username: event.target.value });
            }}
          />
        </div>
      ) : null}
    </div>
  );
}
