import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  LoaderChannel,
  ModpackListItem,
  ModpackSource,
  ModpackVersionItem,
  PlatformId,
} from "@/features/new-instance/model/wizard";
import type { AccountProfile } from "@/lib/generated/AccountProfile";
import type { DownloadJobOperation } from "@/lib/generated/DownloadJobOperation";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";
import type { GameExit } from "@/lib/generated/GameExit";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";
import type { InstanceContentUpdate } from "@/lib/generated/InstanceContentUpdate";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";
import type { InstanceFsEntry } from "@/lib/generated/InstanceFsEntry";
import type { InstanceSettings } from "@/lib/generated/InstanceSettings";
import type { JavaDetections } from "@/lib/generated/JavaDetections";
import type { JavaPaths } from "@/lib/generated/JavaPaths";
import type { JavaStatus } from "@/lib/generated/JavaStatus";
import type { Settings } from "@/lib/generated/Settings";
import type { SettingsSnapshot } from "@/lib/generated/SettingsSnapshot";
import type { StartupPreviewScenario } from "@/lib/generated/StartupPreviewScenario";
import type { StartupProgress } from "@/lib/generated/StartupProgress";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

export function listInstances(): Promise<InstanceEntry[]> {
  return invoke("list_instances");
}

export function getStartupProgress(): Promise<StartupProgress> {
  return invoke("get_startup_progress");
}
export function openStartupPreview(
  scenario: StartupPreviewScenario,
): Promise<void> {
  return invoke("open_startup_preview", { scenario });
}
export function restartStartupPreview(
  scenario: StartupPreviewScenario,
): Promise<void> {
  return invoke("restart_startup_preview", { scenario });
}
export function requestUpdateRestart(): Promise<void> {
  return invoke("request_update_restart");
}

export function listDownloadJobs(): Promise<DownloadJobSnapshot[]> {
  return invoke("list_download_jobs");
}

export function prepareInstanceDownload(
  folder: string,
  operation: DownloadJobOperation = "install",
): Promise<DownloadJobSnapshot> {
  return invoke("prepare_instance_download", { folder, operation });
}

export function pauseDownloadJob(jobId: string): Promise<DownloadJobSnapshot> {
  return invoke("pause_download_job", { jobId });
}

export function resumeDownloadJob(jobId: string): Promise<DownloadJobSnapshot> {
  return invoke("resume_download_job", { jobId });
}

export function cancelDownloadJob(jobId: string): Promise<DownloadJobSnapshot> {
  return invoke("cancel_download_job", { jobId });
}

export function createInstance(
  name: string,
  versionId: string,
): Promise<InstanceEntry> {
  return invoke("create_instance", { name, versionId });
}

export function listLoaderVersions(
  platform: PlatformId | string,
  gameVersion: string,
  channel: LoaderChannel,
): Promise<string[]> {
  return invoke("list_loader_versions", { platform, gameVersion, channel });
}

export function createInstanceWithPlatform(input: {
  name: string;
  platform: PlatformId;
  gameVersion: string;
  loaderChannel: LoaderChannel;
  loaderVersion: string | null;
}): Promise<InstanceEntry> {
  return invoke("create_instance_with_platform", {
    name: input.name,
    platform: input.platform,
    gameVersion: input.gameVersion,
    loaderChannel: input.loaderChannel,
    loaderVersion: input.loaderVersion,
  });
}

export function searchModpacks(query: string): Promise<ModpackListItem[]> {
  return invoke("search_modpacks", { query });
}

export function listModpackVersions(
  source: ModpackSource,
  projectId: string,
): Promise<ModpackVersionItem[]> {
  return invoke("list_modpack_versions", { source, projectId });
}

export function createInstanceFromModpack(input: {
  name: string;
  source: ModpackSource;
  projectId: string;
  versionId: string;
  iconUrl?: string | null;
}): Promise<InstanceEntry> {
  return invoke("create_instance_from_modpack", {
    name: input.name,
    source: input.source,
    projectId: input.projectId,
    versionId: input.versionId,
    iconUrl: input.iconUrl ?? null,
  });
}

export function renameInstance(
  folder: string,
  name: string,
): Promise<InstanceEntry> {
  return invoke("rename_instance", { folder, name });
}

export function duplicateInstance(
  folder: string,
  name: string,
): Promise<InstanceEntry> {
  return invoke("duplicate_instance", { folder, name });
}

export function deleteInstance(folder: string): Promise<void> {
  return invoke("delete_instance", { folder });
}

export function saveInstance(
  folder: string,
  settings: InstanceSettings,
): Promise<InstanceEntry> {
  return invoke("save_instance", { folder, settings });
}

export function setInstanceVersion(
  folder: string,
  versionId: string,
): Promise<InstanceEntry> {
  return invoke("set_instance_version", { folder, versionId });
}

export function openInstanceFolder(folder: string): Promise<void> {
  return invoke("open_instance_folder", { folder });
}

export function listInstanceEntries(
  folder: string,
  relativePath: string,
): Promise<InstanceFsEntry[]> {
  return invoke("list_instance_entries", { folder, relativePath });
}

export async function listInstanceContent(
  folder: string,
  refresh = false,
  onUpdate?: (entries: InstanceContentEntry[], initial: boolean) => void,
  signal?: AbortSignal,
): Promise<InstanceContentEntry[]> {
  const requestId = crypto.randomUUID();
  let stopped = false;
  let initial = true;
  const unsubscribe = onUpdate
    ? await listen<InstanceContentUpdate>(
        "instance-content-updated",
        ({ payload }) => {
          if (
            stopped ||
            signal?.aborted ||
            payload.requestId !== requestId ||
            payload.instanceFolder !== folder
          )
            return;
          onUpdate(payload.entries, initial);
          initial = false;
        },
      )
    : undefined;
  const stop = () => {
    if (stopped) return;
    stopped = true;
    unsubscribe?.();
  };
  signal?.addEventListener("abort", stop, { once: true });
  try {
    if (signal?.aborted)
      throw new DOMException("Content request aborted", "AbortError");
    return await invoke("list_instance_content", {
      folder,
      refresh,
      requestId,
    });
  } finally {
    stop();
    signal?.removeEventListener("abort", stop);
  }
}

export function setInstanceContentEnabled(
  folder: string,
  kind: InstanceContentKind,
  name: string,
  enabled: boolean,
): Promise<void> {
  return invoke("set_instance_content_enabled", {
    folder,
    kind,
    name,
    enabled,
  });
}

export function removeInstanceContent(
  folder: string,
  kind: InstanceContentKind,
  name: string,
  enabled: boolean,
): Promise<void> {
  return invoke("remove_instance_content", {
    folder,
    kind,
    name,
    enabled,
  });
}

export function openRepository(): Promise<void> {
  return invoke("open_repository");
}

export function getLastPlayedFolder(): Promise<string | null> {
  return invoke("get_last_played_folder");
}

export function setLastPlayedFolder(folder: string): Promise<void> {
  return invoke("set_last_played_folder", { folder });
}

export function touchLastOpened(folder: string): Promise<InstanceEntry> {
  return invoke("touch_last_opened", { folder });
}

export function clearLastPlayedFolder(): Promise<void> {
  return invoke("clear_last_played_folder");
}

export function listMicrosoftAccounts(): Promise<AccountProfile[]> {
  return invoke("list_microsoft_accounts");
}

export function getPlayerHead(skinUrl: string | null): Promise<string> {
  return invoke("get_player_head", { skinUrl });
}

export function activeMicrosoftAccount(): Promise<AccountProfile | null> {
  return invoke("active_microsoft_account");
}

export function beginMicrosoftLogin(): Promise<AccountProfile> {
  return invoke("begin_microsoft_login");
}

export function activateOfflineAccount(): Promise<void> {
  return invoke("use_offline_account");
}

export function setActiveMicrosoftAccount(
  uuid: string,
): Promise<AccountProfile> {
  return invoke("set_active_microsoft_account", { uuid });
}

export function logoutMicrosoftAccount(uuid: string): Promise<void> {
  return invoke("logout_microsoft_account", { uuid });
}

export function getSettings(): Promise<SettingsSnapshot> {
  return invoke("get_settings");
}

export function customTitleBarActive(): Promise<boolean> {
  return invoke("custom_title_bar_active");
}

export function restartApp(): Promise<void> {
  return invoke("restart_app");
}

export function saveSettings(settings: Settings): Promise<Settings> {
  return invoke("save_settings", { settings });
}

export function runtimeJava(): Promise<JavaPaths> {
  return invoke("runtime_java");
}

export function javaInstallationStatus(paths: JavaPaths): Promise<JavaStatus> {
  return invoke("java_installation_status", { paths });
}

export function detectJavaInstallations(): Promise<JavaDetections> {
  return invoke("detect_java_installations");
}

export function installRecommendedJava(major: number): Promise<string | null> {
  return invoke("install_recommended_java", { major });
}

export function browseJava(path: string): Promise<string | null> {
  return invoke("browse_java", { path });
}

export function browseApplicationDirectory(
  path: string,
): Promise<string | null> {
  return invoke("browse_application_directory", { path });
}

export function listVersions(): Promise<VersionSummary[]> {
  return invoke("list_versions");
}

export function isVersionReady(versionId: string): Promise<boolean> {
  return invoke("is_version_ready_cmd", { versionId });
}

export function installVersion(versionId: string): Promise<string> {
  return invoke("install_version", { versionId });
}

export function repairInstance(folder: string): Promise<DownloadJobSnapshot> {
  return invoke("repair_instance", { folder });
}

export function installModpackFiles(folder: string): Promise<void> {
  return invoke("install_modpack_files", { folder });
}

export function launchVersion(
  folder: string,
  username: string,
): Promise<GameExit> {
  return invoke("launch", { folder, username });
}

export function listRunningGames(): Promise<string[]> {
  return invoke("list_running_games");
}

export function stopGame(folder: string): Promise<void> {
  return invoke("stop_game", { folder });
}
