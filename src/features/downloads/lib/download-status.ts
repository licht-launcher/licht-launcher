import type { DownloadComponent } from "@/lib/generated/DownloadComponent";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";
import type { DownloadJobStatus } from "@/lib/generated/DownloadJobStatus";
import type { DownloadPhase } from "@/lib/generated/DownloadPhase";

const components: Record<DownloadComponent, string> = {
  minecraft: "Minecraft",
  java: "Java",
  fabric: "Fabric",
  quilt: "Quilt",
  forge: "Forge",
  forgeprofile: "Forge profile",
  neoforge: "NeoForge",
  neoforgeprofile: "NeoForge profile",
  modpack: "Modpack files",
  modpackarchive: "Modpack archive",
};

const phases: Record<DownloadPhase, string> = {
  preparing: "Preparing",
  downloading: "Downloading",
  verifying: "Verifying",
  extracting: "Extracting",
  installing: "Installing",
};

const states: Record<Exclude<DownloadJobStatus, "running">, string> = {
  queued: "Queued",
  paused: "Paused",
  interrupted: "Interrupted",
  cancelling: "Cancellation pending",
  failed: "Failed",
  completed: "Completed",
  cancelled: "Cancelled",
};

export function downloadStatusLabel(job: DownloadJobSnapshot): string {
  return job.status === "running" ? phases[job.phase] : states[job.status];
}

export function downloadStatusText(job: DownloadJobSnapshot): string {
  const label = downloadStatusLabel(job);
  return job.component === null
    ? label
    : `${label} · ${components[job.component]}`;
}
