import { ShieldCheckIcon, SlidersHorizontalIcon, ZapIcon } from "lucide-react";
import { Fragment, type ReactNode } from "react";

import { ChoiceCard } from "@/components/ui/choice-card";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { LoaderChannel } from "@/features/new-instance/model/create-state";

const CHANNELS: { id: LoaderChannel; label: string; icon: ReactNode }[] = [
  {
    id: "stable",
    label: "Stable",
    icon: <ShieldCheckIcon aria-hidden strokeWidth={2} />,
  },
  {
    id: "latest",
    label: "Latest",
    icon: <ZapIcon aria-hidden strokeWidth={2} />,
  },
  {
    id: "manual",
    label: "Other",
    icon: <SlidersHorizontalIcon aria-hidden strokeWidth={2} />,
  },
];

/** Why a channel cannot be chosen, or `undefined` when it can. */
function unavailableReason(
  channel: LoaderChannel,
  {
    loader,
    gameVersion,
    stableMissing,
    noBuilds,
    failed,
  }: {
    loader: string;
    gameVersion: string;
    stableMissing: boolean;
    noBuilds: boolean;
    failed: boolean;
  },
): string | undefined {
  if (failed) {
    return `Could not check ${loader} builds for ${gameVersion}.`;
  }
  if (noBuilds) {
    return `${loader} has no build for ${gameVersion}.`;
  }
  if (channel === "stable" && stableMissing) {
    return `No stable ${loader} build for ${gameVersion}.`;
  }
  return undefined;
}

export function ChannelPicker({
  value,
  loader,
  gameVersion,
  stableMissing,
  noBuilds,
  failed,
  onChange,
}: {
  value: LoaderChannel;
  /** The loader's display name, such as "Quilt". */
  loader: string;
  gameVersion: string;
  /** The loader has no stable build for this game version. */
  stableMissing: boolean;
  /** The loader has no build at all for this game version. */
  noBuilds: boolean;
  /** The builds could not be checked (network or service error). */
  failed: boolean;
  onChange: (channel: LoaderChannel) => void;
}) {
  return (
    <TooltipProvider delayDuration={300}>
      <fieldset className="flex min-w-0 flex-wrap gap-2">
        <legend className="sr-only">Loader channel</legend>
        {CHANNELS.map((channel) => {
          const reason = unavailableReason(channel.id, {
            loader,
            gameVersion,
            stableMissing,
            noBuilds,
            failed,
          });
          const card = (
            <ChoiceCard
              className="w-auto"
              title={channel.label}
              icon={channel.icon}
              iconNeutral
              selected={value === channel.id}
              disabled={reason !== undefined}
              onSelect={() => {
                onChange(channel.id);
              }}
            />
          );
          if (reason === undefined) {
            return <Fragment key={channel.id}>{card}</Fragment>;
          }
          return (
            <Tooltip key={channel.id}>
              <TooltipTrigger asChild>
                {/* A disabled button gets no hover or focus, so this wrapper does. */}
                <span
                  // biome-ignore lint/a11y/noNoninteractiveTabindex: lets keyboard users reach the reason a channel is disabled.
                  tabIndex={0}
                  className="rounded-lg outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
                >
                  {card}
                </span>
              </TooltipTrigger>
              <TooltipContent>{reason}</TooltipContent>
            </Tooltip>
          );
        })}
      </fieldset>
    </TooltipProvider>
  );
}
