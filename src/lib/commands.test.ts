import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  browseApplicationDirectory,
  browseJava,
  createInstance,
  deleteInstance,
  detectJavaInstallations,
  duplicateInstance,
  getSettings,
  installRecommendedJava,
  installVersion,
  javaInstallationStatus,
  launchVersion,
  listInstanceEntries,
  listInstances,
  listRunningGames,
  listVersions,
  openInstanceFolder,
  openRepository,
  renameInstance,
  runtimeJava,
  saveInstance,
  saveSettings,
  setInstanceVersion,
  stopGame,
} from "@/lib/commands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("launcher commands", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  it("lists instances without arguments", async () => {
    await listInstances();
    expect(invoke).toHaveBeenCalledWith("list_instances");
  });

  it("creates, renames, duplicates, and deletes an instance", async () => {
    await createInstance("Survival", "1.20.1");
    expect(invoke).toHaveBeenCalledWith("create_instance", {
      name: "Survival",
      versionId: "1.20.1",
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

  it("opens the repository", async () => {
    await openRepository();
    expect(invoke).toHaveBeenCalledWith("open_repository");
  });

  it("reads settings without arguments", async () => {
    await getSettings();
    expect(invoke).toHaveBeenCalledWith("get_settings");
  });

  it("saves settings and asks for Java and folders", async () => {
    const settings = {
      schema: 1,
      theme: "dark",
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
      disableCustomTitleBar: false,
      dataDirectory: null,
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

  it("installs one version by id", async () => {
    await installVersion("1.20.1");
    expect(invoke).toHaveBeenCalledWith("install_version", {
      versionId: "1.20.1",
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
