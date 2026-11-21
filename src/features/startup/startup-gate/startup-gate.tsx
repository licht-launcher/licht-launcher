import type { ReactNode } from "react";
import { useStartupProgress } from "@/features/startup/hooks/use-startup-progress";

/**
 * The launcher window shows startup and update progress itself, then swaps to
 * the launcher once its services are ready, so no second window opens.
 */
export function StartupGate({
  fallback,
  children,
}: {
  fallback: ReactNode;
  children: ReactNode;
}) {
  const startup = useStartupProgress(true);
  return startup.data?.phase === "ready" ? children : fallback;
}
