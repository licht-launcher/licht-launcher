import { invoke } from "@tauri-apps/api/core";

import type { AccountProfile } from "@/lib/generated/AccountProfile";
import type { GameExit } from "@/lib/generated/GameExit";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";
import type { InstanceFsEntry } from "@/lib/generated/InstanceFsEntry";
import type { InstanceSettings } from "@/lib/generated/InstanceSettings";
import type { JavaDetections } from "@/lib/generated/JavaDetections";
import type { JavaPaths } from "@/lib/generated/JavaPaths";
import type { JavaStatus } from "@/lib/generated/JavaStatus";
import type { Settings } from "@/lib/generated/Settings";
import type { SettingsSnapshot } from "@/lib/generated/SettingsSnapshot";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

export function listInstances(): Promise<InstanceEntry[]> {
  return invoke("list_instances");
}

export function createInstance(
  name: string,
  versionId: string,
): Promise<InstanceEntry> {
  return invoke("create_instance", { name, versionId });
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

export function openRepository(): Promise<void> {
  return invoke("open_repository");
}

export function getLastPlayedFolder(): Promise<string | null> {
  return invoke("get_last_played_folder");
}

export function setLastPlayedFolder(folder: string): Promise<void> {
  return invoke("set_last_played_folder", { folder });
}

export function clearLastPlayedFolder(): Promise<void> {
  return invoke("clear_last_played_folder");
}

export function listMicrosoftAccounts(): Promise<AccountProfile[]> {
  return invoke("list_microsoft_accounts");
}

export function activeMicrosoftAccount(): Promise<AccountProfile | null> {
  return invoke("active_microsoft_account");
}

export function beginMicrosoftLogin(): Promise<AccountProfile> {
  return invoke("begin_microsoft_login");
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

export function installRecommendedJava(major: number): Promise<string> {
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

export function installVersion(versionId: string): Promise<string> {
  return invoke("install_version", { versionId });
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
