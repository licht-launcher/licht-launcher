import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { buildDiagnostics } from "@/features/settings/fixtures/build-diagnostics";
import { appUpdateState } from "@/features/updater/fixtures/state";
import {
  activateOfflineAccount,
  activeMicrosoftAccount,
  beginMicrosoftLogin,
  browseApplicationDirectory,
  browseJava,
  cancelDownloadJob,
  checkAppUpdate,
  clearLastPlayedFolder,
  closeStartupPreview,
  createInstance,
  createInstanceFromModpack,
  createInstanceWithPlatform,
  customTitleBarActive,
  deleteInstance,
  detectJavaInstallations,
  downloadAppUpdate,
  duplicateInstance,
  getAppUpdateState,
  getBuildDiagnostics,
  getLastPlayedFolder,
  getPlayerHead,
  getSettings,
  getStartupProgress,
  installAppUpdate,
  installModpackFiles,
  installRecommendedJava,
  installVersion,
  isStartupPreviewOpen,
  isVersionReady,
  javaInstallationStatus,
  launchVersion,
  listDownloadJobs,
  listInstanceContent,
  listInstanceEntries,
  listInstances,
  listLoaderVersions,
  listMicrosoftAccounts,
  listModpackVersions,
  listRunningGames,
  listVersions,
  logoutMicrosoftAccount,
  openFeedback,
  openInstanceFolder,
  openRepository,
  openStartupPreview,
  pauseDownloadJob,
  prepareInstanceDownload,
  quitApp,
  renameInstance,
  repairInstance,
  requestUpdateRestart,
  restartApp,
  restartStartupPreview,
  resumeDownloadJob,
  runtimeJava,
  saveInstance,
  saveSettings,
  searchModpacks,
  setActiveMicrosoftAccount,
  setInstanceVersion,
  setLastPlayedFolder,
  stopGame,
  touchLastOpened,
} from "@/lib/commands";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentUpdate } from "@/lib/generated/InstanceContentUpdate";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const { contentListeners, unsubscribe } = vi.hoisted(() => ({
  contentListeners: new Set<
    (event: { payload: InstanceContentUpdate }) => void
  >(),
  unsubscribe: vi.fn(),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(
    async (
      _name: string,
      handler: (event: { payload: InstanceContentUpdate }) => void,
    ) => {
      contentListeners.add(handler);
      return () => {
        unsubscribe();
        contentListeners.delete(handler);
      };
    },
  ),
}));

describe("launcher commands", () => {
  it("loads and validates native build diagnostics", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(buildDiagnostics);
    await expect(getBuildDiagnostics()).resolves.toEqual(buildDiagnostics);
    expect(invoke).toHaveBeenCalledWith("get_build_diagnostics");
    vi.mocked(invoke).mockResolvedValueOnce({ debug: "false" });
    await expect(getBuildDiagnostics()).rejects.toThrow(
      "Invalid build diagnostics",
    );
  });
  it("routes signed application-update operations to the native coordinator", async () => {
    const state = appUpdateState({ phase: "ready", version: "0.4.0" });
    vi.mocked(invoke).mockResolvedValueOnce(state);
    expect(await getAppUpdateState()).toEqual(state);
    expect(invoke).toHaveBeenCalledWith("get_app_update_state");
    await checkAppUpdate();
    await downloadAppUpdate();
    await installAppUpdate();
    expect(invoke).toHaveBeenCalledWith("check_app_update");
    expect(invoke).toHaveBeenCalledWith("download_app_update");
    expect(invoke).toHaveBeenCalledWith("install_app_update");
  });
  it("reads and closes the single startup preview through native commands", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(true);
    expect(await isStartupPreviewOpen()).toBe(true);
    expect(invoke).toHaveBeenCalledWith("is_startup_preview_open");
    await closeStartupPreview();
    expect(invoke).toHaveBeenCalledWith("close_startup_preview");
  });

  it("rejects an invalid native preview state", async () => {
    vi.mocked(invoke).mockResolvedValueOnce("yes");
    await expect(isStartupPreviewOpen()).rejects.toThrow(
      "Invalid startup preview window state",
    );
  });
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue(undefined);
    contentListeners.clear();
    unsubscribe.mockClear();
    vi.mocked(listen).mockClear();
  });

  it("lists instances without arguments", async () => {
    await listInstances();
    expect(invoke).toHaveBeenCalledWith("list_instances");
  });

  it("routes startup status, isolated previews and update restart to the native coordinator", async () => {
    await getStartupProgress();
    expect(invoke).toHaveBeenCalledWith("get_startup_progress");
    await openStartupPreview("migration");
    expect(invoke).toHaveBeenCalledWith("open_startup_preview", {
      scenario: "migration",
    });
    await restartStartupPreview("updateAndMigration");
    expect(invoke).toHaveBeenCalledWith("restart_startup_preview", {
      scenario: "updateAndMigration",
    });
    await requestUpdateRestart();
    expect(invoke).toHaveBeenCalledWith("request_update_restart");
  });

  it("returns persisted download snapshots without requesting a new transfer", async () => {
    const jobs: DownloadJobSnapshot[] = [
      {
        id: "17",
        instanceFolder: "Survival",
        versionId: "1.21.1",
        operation: "install",
        status: "interrupted",
        component: "java",
        phase: "downloading",
        progress: {
          finished: 0,
          failed: 0,
          total: 1,
          bytesDone: 5,
          bytesTotal: 11,
        },
        createdAt: 1_700_000_000_000,
        updatedAt: 1_700_000_001_000,
      },
    ];
    vi.mocked(invoke).mockResolvedValueOnce(jobs);

    await expect(listDownloadJobs()).resolves.toEqual(jobs);
    expect(invoke).toHaveBeenCalledExactlyOnceWith("list_download_jobs");
  });

  it("propagates download job listing failures", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(
      new Error("database operation failed"),
    );

    await expect(listDownloadJobs()).rejects.toThrow(
      "database operation failed",
    );
  });

  it("routes preparation and per-job controls through the managed download commands", async () => {
    await prepareInstanceDownload("Survival", "repair");
    expect(invoke).toHaveBeenCalledWith("prepare_instance_download", {
      folder: "Survival",
      operation: "repair",
    });
    await pauseDownloadJob("17");
    expect(invoke).toHaveBeenCalledWith("pause_download_job", { jobId: "17" });
    await resumeDownloadJob("17");
    expect(invoke).toHaveBeenCalledWith("resume_download_job", { jobId: "17" });
    await cancelDownloadJob("17");
    expect(invoke).toHaveBeenCalledWith("cancel_download_job", { jobId: "17" });
  });

  it("creates, renames, duplicates, and deletes an instance", async () => {
    await createInstance("Survival", "1.20.1");
    expect(invoke).toHaveBeenCalledWith("create_instance", {
      name: "Survival",
      versionId: "1.20.1",
    });
    await createInstanceWithPlatform({
      name: "Fabric",
      platform: "fabric",
      gameVersion: "1.20.1",
      loaderChannel: "stable",
      loaderVersion: null,
    });
    expect(invoke).toHaveBeenCalledWith("create_instance_with_platform", {
      name: "Fabric",
      platform: "fabric",
      gameVersion: "1.20.1",
      loaderChannel: "stable",
      loaderVersion: null,
    });
    await listLoaderVersions("fabric", "1.20.1", "stable");
    expect(invoke).toHaveBeenCalledWith("list_loader_versions", {
      platform: "fabric",
      gameVersion: "1.20.1",
      channel: "stable",
    });
    await searchModpacks("cobble");
    expect(invoke).toHaveBeenCalledWith("search_modpacks", { query: "cobble" });
    await listModpackVersions("modrinth", "ABC");
    expect(invoke).toHaveBeenCalledWith("list_modpack_versions", {
      source: "modrinth",
      projectId: "ABC",
    });
    await createInstanceFromModpack({
      name: "Cobblemon",
      source: "modrinth",
      projectId: "ABC",
      versionId: "v1",
    });
    expect(invoke).toHaveBeenCalledWith("create_instance_from_modpack", {
      name: "Cobblemon",
      source: "modrinth",
      projectId: "ABC",
      versionId: "v1",
      iconUrl: null,
    });
    await renameInstance("Survival", "Classic");
    expect(invoke).toHaveBeenCalledWith("rename_instance", {
      folder: "Survival",
      name: "Classic",
    });
    await duplicateInstance("Survival", "Survival copy");
    expect(invoke).toHaveBeenCalledWith("duplicate_instance", {
      folder: "Survival",
      name: "Survival copy",
    });
    await deleteInstance("Survival");
    expect(invoke).toHaveBeenCalledWith("delete_instance", {
      folder: "Survival",
    });
    const settings = {
      overrideWindow: true,
      overrideMemory: false,
      overrideJava: false,
      overrideJvmArguments: false,
      javaPath: null,
      maxMemoryMb: 4096,
      jvmArguments: [],
      fullscreen: true,
      width: 1280,
      height: 720,
    };
    await saveInstance("Survival", settings);
    expect(invoke).toHaveBeenCalledWith("save_instance", {
      folder: "Survival",
      settings,
    });
    await setInstanceVersion("Survival", "1.21.1");
    expect(invoke).toHaveBeenCalledWith("set_instance_version", {
      folder: "Survival",
      versionId: "1.21.1",
    });
  });

  it("opens one instance folder by its id", async () => {
    await openInstanceFolder("Survival");
    expect(invoke).toHaveBeenCalledWith("open_instance_folder", {
      folder: "Survival",
    });
  });

  it("lists instance folder entries for a relative path", async () => {
    await listInstanceEntries("Survival", "saves");
    expect(invoke).toHaveBeenCalledWith("list_instance_entries", {
      folder: "Survival",
      relativePath: "saves",
    });
  });

  it("subscribes before loading content and routes only matching request events", async () => {
    const updates = vi.fn();
    const entries: InstanceContentEntry[] = [
      {
        name: "one.jar",
        kind: "mods",
        enabled: true,
        size: 3,
        title: "One",
        version: null,
        iconUrl: null,
      },
    ];
    vi.mocked(invoke).mockImplementationOnce(async (_command, args) => {
      expect(contentListeners.size).toBe(1);
      const request = args as { requestId: string };
      for (const handler of contentListeners) {
        handler({
          payload: {
            requestId: "older-request",
            instanceFolder: "classic",
            entries,
          },
        });
        handler({
          payload: {
            requestId: request.requestId,
            instanceFolder: "other",
            entries,
          },
        });
        handler({
          payload: {
            requestId: request.requestId,
            instanceFolder: "classic",
            entries,
          },
        });
        handler({
          payload: {
            requestId: request.requestId,
            instanceFolder: "classic",
            entries: [{ ...entries[0], version: "1.0" }],
          },
        });
      }
      return entries;
    });
    await expect(
      listInstanceContent("classic", false, updates),
    ).resolves.toEqual(entries);
    expect(updates.mock.calls).toEqual([
      [entries, true],
      [[{ ...entries[0], version: "1.0" }], false],
    ]);
    expect(invoke).toHaveBeenCalledWith("list_instance_content", {
      folder: "classic",
      refresh: false,
      requestId: expect.any(String),
    });
    expect(unsubscribe).toHaveBeenCalledTimes(1);
    expect(contentListeners.size).toBe(0);
  });

  it("cleans up the content subscription and preserves command failures", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("metadata failed"));
    await expect(
      listInstanceContent("classic", false, vi.fn()),
    ).rejects.toThrow("metadata failed");
    expect(contentListeners.size).toBe(0);
    expect(unsubscribe).toHaveBeenCalledTimes(1);
  });

  it("stops content updates on abort, including events already queued by the bridge", async () => {
    const updates = vi.fn();
    const controller = new AbortController();
    vi.mocked(invoke).mockImplementationOnce(async (_command, args) => {
      const handler = [...contentListeners][0];
      const request = args as { requestId: string };
      controller.abort();
      handler({
        payload: {
          requestId: request.requestId,
          instanceFolder: "classic",
          entries: [],
        },
      });
      return [];
    });
    await listInstanceContent("classic", true, updates, controller.signal);
    expect(updates).not.toHaveBeenCalled();
    expect(unsubscribe).toHaveBeenCalledTimes(1);
    expect(contentListeners.size).toBe(0);
  });

  it("does not start an already aborted content request", async () => {
    const controller = new AbortController();
    controller.abort();
    await expect(
      listInstanceContent("classic", false, vi.fn(), controller.signal),
    ).rejects.toThrow("Content request aborted");
    expect(invoke).not.toHaveBeenCalled();
    expect(contentListeners.size).toBe(0);
  });

  it("reports content event subscription failures before invoking the command", async () => {
    vi.mocked(listen).mockRejectedValueOnce(
      new Error("event bridge unavailable"),
    );
    await expect(
      listInstanceContent("classic", false, vi.fn()),
    ).rejects.toThrow("event bridge unavailable");
    expect(invoke).not.toHaveBeenCalled();
  });

  it("opens the repository", async () => {
    await openRepository();
    expect(invoke).toHaveBeenCalledWith("open_repository");
  });

  it("opens the feedback form through the native command", async () => {
    await openFeedback();
    expect(invoke).toHaveBeenCalledWith("open_feedback");
  });

  it("reads the cached player head through the native command", async () => {
    const path = "C:\\Licht\\cache\\player-heads\\default.png";
    vi.mocked(invoke).mockResolvedValueOnce(path);
    await expect(getPlayerHead(null)).resolves.toBe(path);
    expect(invoke).toHaveBeenCalledWith("get_player_head", { skinUrl: null });
  });

  it("selects offline mode through the native command without logging out saved accounts", async () => {
    await activateOfflineAccount();
    expect(invoke).toHaveBeenCalledWith("use_offline_account");
    expect(invoke).not.toHaveBeenCalledWith(
      "logout_microsoft_account",
      expect.anything(),
    );
  });

  it("reads and writes the last played folder", async () => {
    await getLastPlayedFolder();
    expect(invoke).toHaveBeenCalledWith("get_last_played_folder");
    await setLastPlayedFolder("classic");
    expect(invoke).toHaveBeenCalledWith("set_last_played_folder", {
      folder: "classic",
    });
    await touchLastOpened("classic");
    expect(invoke).toHaveBeenCalledWith("touch_last_opened", {
      folder: "classic",
    });
    await clearLastPlayedFolder();
    expect(invoke).toHaveBeenCalledWith("clear_last_played_folder");
  });

  it("lists and signs in Microsoft accounts", async () => {
    await listMicrosoftAccounts();
    expect(invoke).toHaveBeenCalledWith("list_microsoft_accounts");
    await activeMicrosoftAccount();
    expect(invoke).toHaveBeenCalledWith("active_microsoft_account");
    await beginMicrosoftLogin();
    expect(invoke).toHaveBeenCalledWith("begin_microsoft_login");
    await setActiveMicrosoftAccount("abc");
    expect(invoke).toHaveBeenCalledWith("set_active_microsoft_account", {
      uuid: "abc",
    });
    await logoutMicrosoftAccount("abc");
    expect(invoke).toHaveBeenCalledWith("logout_microsoft_account", {
      uuid: "abc",
    });
  });

  it("reads settings without arguments", async () => {
    await getSettings();
    expect(invoke).toHaveBeenCalledWith("get_settings");
  });

  it("reads the applied title bar mode and can request a restart", async () => {
    await customTitleBarActive();
    expect(invoke).toHaveBeenCalledWith("custom_title_bar_active");
    await restartApp();
    expect(invoke).toHaveBeenCalledWith("restart_app");
  });

  it("requests application exit independently of window-close behavior", async () => {
    await quitApp();
    expect(invoke).toHaveBeenCalledWith("quit_app");
  });

  it("saves settings and asks for Java and folders", async () => {
    const settings = {
      schema: 1,
      theme: "dark" as const,
      username: "Steve",
      fullscreen: false,
      width: null,
      height: null,
      maxMemoryMb: 2048,
      jvmArguments: [],
      java25: null,
      java21: null,
      java17: null,
      java8: null,
      downloadConcurrency: 4,
      writeConcurrency: 8,
      downloadSpeedLimitBytesPerSecond: null,
      disableCustomTitleBar: false,
      dataDirectory: null,
      automaticUpdates: true,
      language: "en" as const,
      hardwareAcceleration: true,
      minimizeToTray: false,
      minimizeOnGameStart: false,
      restoreOnGameExit: false,
    };
    await saveSettings(settings);
    expect(invoke).toHaveBeenCalledWith("save_settings", { settings });
    await detectJavaInstallations();
    expect(invoke).toHaveBeenCalledWith("detect_java_installations");
    await runtimeJava();
    expect(invoke).toHaveBeenCalledWith("runtime_java");
    const paths = {
      java25: null,
      java21: "C:\\runtime\\java.exe",
      java17: null,
      java8: null,
    };
    await javaInstallationStatus(paths);
    expect(invoke).toHaveBeenCalledWith("java_installation_status", { paths });
    await installRecommendedJava(21);
    expect(invoke).toHaveBeenCalledWith("install_recommended_java", {
      major: 21,
    });
    await browseJava("C:\\runtime\\java.exe");
    expect(invoke).toHaveBeenCalledWith("browse_java", {
      path: "C:\\runtime\\java.exe",
    });
    await browseApplicationDirectory("D:\\Games");
    expect(invoke).toHaveBeenCalledWith("browse_application_directory", {
      path: "D:\\Games",
    });
  });

  it("lists versions without arguments", async () => {
    await listVersions();
    expect(invoke).toHaveBeenCalledWith("list_versions");
  });

  it("checks whether a version is ready to launch", async () => {
    await isVersionReady("1.20.1");
    expect(invoke).toHaveBeenCalledWith("is_version_ready_cmd", {
      versionId: "1.20.1",
    });
  });

  it("installs one version by id", async () => {
    await installVersion("1.20.1");
    expect(invoke).toHaveBeenCalledWith("install_version", {
      versionId: "1.20.1",
    });
  });

  it("repairs an instance folder", async () => {
    await repairInstance("classic");
    expect(invoke).toHaveBeenCalledWith("repair_instance", {
      folder: "classic",
    });
  });

  it("installs modpack files for an instance folder", async () => {
    await installModpackFiles("Cobblemon");
    expect(invoke).toHaveBeenCalledWith("install_modpack_files", {
      folder: "Cobblemon",
    });
  });

  it("launches with the offline username", async () => {
    await launchVersion("classic", "Mateus");
    expect(invoke).toHaveBeenCalledWith("launch", {
      folder: "classic",
      username: "Mateus",
    });
  });

  it("lists running games", async () => {
    vi.mocked(invoke).mockResolvedValue(["classic"]);
    await expect(listRunningGames()).resolves.toEqual(["classic"]);
    expect(invoke).toHaveBeenCalledWith("list_running_games");
  });

  it("stops a running game by folder", async () => {
    await stopGame("classic");
    expect(invoke).toHaveBeenCalledWith("stop_game", { folder: "classic" });
  });
});
