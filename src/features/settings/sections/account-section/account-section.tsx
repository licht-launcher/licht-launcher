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

import { accountSectionStyles } from "./account-section.styles";

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
      <div className={accountSectionStyles.row}>
        <div className={accountSectionStyles.rowText}>
          <h4 className={accountSectionStyles.rowLabel}>
            Sign in with Microsoft
          </h4>
          <p className={accountSectionStyles.rowDescription}>
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
              className={accountSectionStyles.spinner}
            />
          ) : (
            <LogInIcon data-icon="inline-start" aria-hidden />
          )}
          {begin.isPending ? "Waiting for browser…" : "Sign in"}
        </Button>
      </div>
      {begin.isPending ? (
        <p className={accountSectionStyles.rowDescription}>
          Complete sign-in in your browser, then return here.
        </p>
      ) : null}
      {loginError !== null ? (
        <p className={accountSectionStyles.error} role="alert">
          {loginError}
        </p>
      ) : null}
      {profiles.length > 0 ? (
        <ul className={accountSectionStyles.profiles}>
          {profiles.map((profile) => {
            const isActive = profile.uuid === activeUuid;
            return (
              <li key={profile.uuid} className={accountSectionStyles.profile}>
                <div className={accountSectionStyles.profileText}>
                  <p className={accountSectionStyles.profileName}>
                    {profile.name}
                  </p>
                  <p className={accountSectionStyles.profileId}>
                    {profile.uuid}
                  </p>
                </div>
                <div className={accountSectionStyles.profileActions}>
                  {isActive ? (
                    <span className={accountSectionStyles.profileStatus}>
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
