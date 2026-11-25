import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import type { ComponentProps } from "react";
import { toast } from "sonner";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Toaster } from "@/components/ui/sonner";
import {
  completedDownload,
  downloadJob,
} from "@/features/downloads/fixtures/job";
import { InstanceSettingsDialog } from "@/features/instance-settings/instance-settings-dialog";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

const { invoke } = vi.hoisted(() => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke,
}));

const entry: InstanceEntry = {
  folder: "Survival",
  name: "Survival",
  versionId: "1.20.1",
  minMemoryMb: 512,
  maxMemoryMb: 2048,
  jvmArguments: [],
  fullscreen: false,
  width: 854,
  height: 480,
  overrideWindow: false,
  overrideMemory: false,
  overrideJava: false,
  overrideJvmArguments: false,
  javaPath: null,
  lastOpenedAt: null,
  iconPath: null,
};

function renderDialog(
  props: Partial<ComponentProps<typeof InstanceSettingsDialog>> = {},
) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <InstanceSettingsDialog
        entry={entry}
        open
        busy={false}
        versionBusy={false}
        onOpenChange={() => {}}
        onOpenGlobalSettings={() => {}}
        onRename={() => {}}
        onChangeBase={() => {}}
        onOpenFolder={() => {}}
        onDuplicate={() => {}}
        onDelete={() => {}}
        {...props}
      />
      <Toaster />
    </QueryClientProvider>,
  );
}

const linkedEntry: InstanceEntry = {
  ...entry,
  installation: {
    source: {
      type: "modpack",
      source: "modrinth",
      projectId: "fo",
      versionId: "v15",
      iconUrl: null,
    },
    state: "ready",
    modpack: {
      name: "Fabulously Optimized",
      author: "robotkoer",
      versionName: "15.0.0-beta.3",
      publishedAt: null,
    },
  },
};

describe("InstanceSettingsDialog", () => {
  afterEach(() => vi.restoreAllMocks());
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockImplementation((command: string) => {
      if (command === "get_settings") {
        return Promise.resolve({
          settings: {
            schema: 1,
            theme: "dark",
            username: "Steve",
            fullscreen: false,
            width: 854,
            height: 480,
            maxMemoryMb: 2048,
            jvmArguments: [],
            java25: null,
            java21: null,
            java17: null,
            java8: null,
            downloadConcurrency: 4,
            writeConcurrency: 8,
            dataDirectory: null,
          },
          applicationDirectory: "C:\\Licht",
          memoryTotalMb: 16384,
          memoryRecommendedMb: 4096,
          memoryMinimumMb: 512,
          memoryMaximumMb: 16384,
          memoryStepMb: 64,
        });
      }
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.20.1", versionType: "release", installed: true },
          { id: "1.21.1", versionType: "release", installed: false },
          { id: "24w14a", versionType: "snapshot", installed: false },
        ]);
      }
      if (command === "get_instance_platform") {
        return Promise.resolve({
          platform: "vanilla",
          gameVersion: "1.20.1",
          loaderVersion: null,
        });
      }
      return Promise.resolve(entry);
    });
  });

  it("opens on General with rename, folder, duplicate, and delete", async () => {
    const onRename = vi.fn();
    const onOpenFolder = vi.fn();
    const onDuplicate = vi.fn();
    const onDelete = vi.fn();
    const onChangeBase = vi.fn();
    renderDialog({
      onRename,
      onOpenFolder,
      onDuplicate,
      onDelete,
      onChangeBase,
    });

    expect(
      await screen.findByRole("button", { name: "General" }),
    ).toHaveAttribute("aria-current", "page");
    expect(
      await screen.findByText("Name, folder, and instance actions."),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Display name for this instance. The folder on disk stays the same.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Edit base" }),
    ).not.toBeInTheDocument();

    const name = screen.getByLabelText("Name");
    fireEvent.change(name, { target: { value: "Hardcore" } });
    fireEvent.blur(name);
    expect(onRename).toHaveBeenCalledWith("Hardcore");

    expect(
      screen.getByRole("heading", { name: "Open folder" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Duplicate" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Delete" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Open folder" }));
    fireEvent.click(screen.getByRole("button", { name: "Duplicate" }));
    fireEvent.click(screen.getByRole("button", { name: "Delete instance" }));
    expect(onOpenFolder).toHaveBeenCalled();
    expect(onDuplicate).toHaveBeenCalled();
    expect(onDelete).toHaveBeenCalled();
    expect(
      screen.queryByRole("button", { name: "Manage" }),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Instance base" }));
    expect(
      await screen.findByText(
        "Platform and game version this instance launches with.",
      ),
    ).toBeInTheDocument();
    expect(await screen.findByText("Vanilla")).toBeInTheDocument();
    expect(screen.getByText("1.20.1")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Edit base" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Change the platform, game version or loader this instance launches with.",
      ),
    ).toBeInTheDocument();
    expect(
      screen
        .getAllByRole("heading", { level: 3 })
        .map((heading) => heading.textContent),
    ).toEqual(["Edit base", "Repair"]);
    expect(invoke).not.toHaveBeenCalledWith("list_versions");

    // The version list is virtualized; give it room to render its options.
    vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(
      function (this: HTMLElement) {
        return this.dataset.slot === "combobox-viewport" ? 256 : 0;
      },
    );
    vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(320);
    fireEvent.click(screen.getByRole("button", { name: "Edit base" }));
    expect(
      await screen.findByRole("dialog", { name: "Edit base" }),
    ).toBeInTheDocument();
    const field = screen.getByRole("combobox", { name: "Game version" });
    await waitFor(() => expect(field).toHaveValue("1.20.1"));
    fireEvent.focus(field);
    fireEvent.change(field, { target: { value: "1.21" } });
    fireEvent.click(await screen.findByRole("option", { name: "1.21.1" }));
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(onChangeBase).toHaveBeenCalledWith({
      platform: "vanilla",
      gameVersion: "1.21.1",
      loaderChannel: "stable",
      loaderVersion: null,
    });
  });

  it("shows the loader and Minecraft version an instance runs", async () => {
    const original = invoke.getMockImplementation();
    if (!original) throw new Error("missing command implementation");
    invoke.mockImplementation((command: string, args?: unknown) =>
      command === "get_instance_platform"
        ? Promise.resolve({
            platform: "fabric",
            gameVersion: "26.3",
            loaderVersion: "0.19.5",
          })
        : original(command, args),
    );
    renderDialog({
      entry: { ...entry, versionId: "fabric-loader-0.19.5-26.3" },
    });

    fireEvent.click(screen.getByRole("button", { name: "Instance base" }));

    expect(await screen.findByText("Fabric 0.19.5")).toBeInTheDocument();
    expect(screen.getByText("26.3")).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("get_instance_platform", {
      versionId: "fabric-loader-0.19.5-26.3",
    });
  });

  it("says when the platform cannot be read", async () => {
    const original = invoke.getMockImplementation();
    if (!original) throw new Error("missing command implementation");
    invoke.mockImplementation((command: string, args?: unknown) =>
      command === "get_instance_platform"
        ? Promise.reject(new Error("invalid loader profile"))
        : original(command, args),
    );
    renderDialog();

    fireEvent.click(screen.getByRole("button", { name: "Instance base" }));

    expect(
      await screen.findByText("Could not read the platform"),
    ).toBeInTheDocument();
  });

  it("reinstalls a store modpack after the user confirms", async () => {
    const original = invoke.getMockImplementation();
    if (!original) throw new Error("missing command implementation");
    invoke.mockImplementation((command: string, args?: unknown) =>
      command === "reinstall_instance_modpack"
        ? Promise.resolve(
            completedDownload({
              instanceFolder: "Survival",
              operation: "reinstall",
            }),
          )
        : original(command, args),
    );
    const success = vi.spyOn(toast, "success");
    renderDialog({ entry: linkedEntry });

    fireEvent.click(
      await screen.findByRole("button", { name: "Instance base" }),
    );
    fireEvent.click(await screen.findByRole("button", { name: "Reinstall" }));
    const confirm = await screen.findByRole("dialog", {
      name: "Reinstall this modpack?",
    });
    expect(invoke).not.toHaveBeenCalledWith(
      "reinstall_instance_modpack",
      expect.anything(),
    );
    fireEvent.click(within(confirm).getByRole("button", { name: "Reinstall" }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("reinstall_instance_modpack", {
        folder: "Survival",
      }),
    );
    await waitFor(() =>
      expect(success).toHaveBeenCalledWith("Modpack reinstalled."),
    );
  });

  it("cannot reinstall a modpack imported from a file", async () => {
    renderDialog({
      entry: {
        ...linkedEntry,
        installation: {
          source: { type: "modpackFile", source: "modrinth" },
          state: "ready",
        },
      },
    });

    fireEvent.click(
      await screen.findByRole("button", { name: "Instance base" }),
    );

    expect(
      await screen.findByText(
        "A modpack imported from a file cannot be reinstalled, because its archive is not kept after install.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Reinstall" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Unlink" })).toBeEnabled();
  });

  it("shows the linked modpack under Instance base only for linked instances", async () => {
    const { unmount } = renderDialog();
    fireEvent.click(
      await screen.findByRole("button", { name: "Instance base" }),
    );
    expect(await screen.findByText("Vanilla")).toBeInTheDocument();
    expect(screen.queryByText("Linked modpack")).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Unlink" }),
    ).not.toBeInTheDocument();
    unmount();

    renderDialog({ entry: linkedEntry });
    fireEvent.click(
      await screen.findByRole("button", { name: "Instance base" }),
    );
    expect(await screen.findByText("Linked modpack")).toBeInTheDocument();
    expect(
      screen.getByText("Fabulously Optimized 15.0.0-beta.3"),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Modpack" }),
    ).not.toBeInTheDocument();
  });

  it("holds the modpack actions while the game or a download runs", async () => {
    renderDialog({ entry: linkedEntry, instanceActive: true });

    fireEvent.click(
      await screen.findByRole("button", { name: "Instance base" }),
    );

    expect(
      await screen.findByRole("button", { name: "Reinstall" }),
    ).toBeDisabled();
    expect(screen.getByRole("button", { name: "Unlink" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Repair" })).toBeEnabled();
  });

  it("unlinks the modpack after the user confirms", async () => {
    const success = vi.spyOn(toast, "success");
    renderDialog({ entry: linkedEntry });

    fireEvent.click(
      await screen.findByRole("button", { name: "Instance base" }),
    );
    expect(
      await screen.findByText("Fabulously Optimized 15.0.0-beta.3"),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Unlink" }));
    const confirm = await screen.findByRole("dialog", {
      name: "Unlink this modpack?",
    });
    expect(invoke).not.toHaveBeenCalledWith(
      "unlink_instance_modpack",
      expect.anything(),
    );
    fireEvent.click(within(confirm).getByRole("button", { name: "Unlink" }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("unlink_instance_modpack", {
        folder: "Survival",
      }),
    );
    await waitFor(() =>
      expect(success).toHaveBeenCalledWith("Modpack unlinked."),
    );
  });

  it("installs another modpack version in place of Edit base", async () => {
    const original = invoke.getMockImplementation();
    if (!original) throw new Error("missing command implementation");
    invoke.mockImplementation((command: string, args?: unknown) => {
      if (command === "list_modpack_versions") {
        return Promise.resolve(
          ["v16", "v15"].map((id) => ({
            id,
            name: `Fabulously Optimized ${id}`,
            gameVersions: ["1.21.1"],
            loaders: ["fabric"],
            versionType: "release",
            publishedAt: null,
            downloads: 0,
          })),
        );
      }
      if (command === "change_instance_modpack_version") {
        return Promise.resolve(
          completedDownload({ operation: "changeVersion" }),
        );
      }
      return original(command, args);
    });
    renderDialog({
      entry: {
        ...entry,
        installation: {
          source: {
            type: "modpack",
            source: "modrinth",
            projectId: "fo",
            versionId: "v15",
            iconUrl: null,
          },
          state: "ready",
        },
      },
    });

    fireEvent.click(screen.getByRole("button", { name: "Instance base" }));

    expect(
      await screen.findByRole("heading", { name: "Modpack version" }),
    ).toBeInTheDocument();
    // Each action sits right under the card it changes.
    expect(
      screen
        .getAllByRole("heading", { level: 3 })
        .map((heading) => heading.textContent),
    ).toEqual([
      "Modpack version",
      "Reinstall modpack",
      "Unlink modpack",
      "Repair",
    ]);
    expect(
      screen.queryByRole("button", { name: "Edit base" }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Repair" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "Change version" }));
    const install = await screen.findByRole("button", {
      name: "Install version",
    });
    await waitFor(() => {
      expect(screen.getByLabelText("Version")).toHaveTextContent(
        "Fabulously Optimized v15",
      );
    });
    expect(install).toBeDisabled();
    fireEvent.click(screen.getByLabelText("Version"));
    fireEvent.click(
      await screen.findByRole("option", { name: "Fabulously Optimized v16" }),
    );
    fireEvent.click(install);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("change_instance_modpack_version", {
        folder: "Survival",
        versionId: "v16",
      });
    });
    expect(
      await screen.findByText("Modpack version changed."),
    ).toBeInTheDocument();
  });

  it("has no other versions for a modpack imported from a file", async () => {
    renderDialog({
      entry: {
        ...entry,
        installation: {
          source: { type: "modpackFile", source: "modrinth" },
          state: "ready",
        },
      },
    });

    fireEvent.click(screen.getByRole("button", { name: "Instance base" }));

    expect(
      await screen.findByText(
        "A modpack imported from a file has no other versions to install.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Change version" }),
    ).toBeDisabled();
  });

  it("repairs the instance from the base section", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "get_settings") {
        return Promise.resolve({
          settings: {
            schema: 1,
            theme: "dark",
            username: "Steve",
            fullscreen: false,
            width: 854,
            height: 480,
            maxMemoryMb: 2048,
            jvmArguments: [],
            java25: null,
            java21: null,
            java17: null,
            java8: null,
            downloadConcurrency: 4,
            writeConcurrency: 8,
            dataDirectory: null,
          },
          applicationDirectory: "C:\\Licht",
          memoryTotalMb: 16384,
          memoryRecommendedMb: 4096,
          memoryMinimumMb: 512,
          memoryMaximumMb: 16384,
          memoryStepMb: 64,
        });
      }
      if (command === "repair_instance") {
        return Promise.resolve(completedDownload({ operation: "repair" }));
      }
      return Promise.resolve(entry);
    });
    renderDialog();
    fireEvent.click(
      await screen.findByRole("button", { name: "Instance base" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Repair" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("repair_instance", {
        folder: "Survival",
      });
    });
    expect(
      await screen.findByText("Instance files were repaired."),
    ).toBeInTheDocument();
  });

  it("does not announce repair success when the task is cancelled", async () => {
    const original = invoke.getMockImplementation();
    if (!original) throw new Error("missing command implementation");
    invoke.mockImplementation((command: string) =>
      command === "repair_instance"
        ? Promise.resolve(
            downloadJob({ status: "cancelled", operation: "repair" }),
          )
        : original(command),
    );
    const success = vi.spyOn(toast, "success");
    renderDialog();
    fireEvent.click(
      await screen.findByRole("button", { name: "Instance base" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Repair" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("repair_instance", {
        folder: "Survival",
      }),
    );
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Repair" })).toBeEnabled(),
    );
    expect(success).not.toHaveBeenCalled();
  });

  it("groups overrides under Override Settings", async () => {
    renderDialog();
    expect(await screen.findByText("Survival")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Open Global Settings" }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Override Settings" }));

    expect(
      await screen.findByText(
        "Turn on a category to use values from this instance instead of global Settings.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Window" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Memory" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Java" })).toBeInTheDocument();
    expect(
      screen.queryByRole("heading", { name: "Arguments" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("switch", { name: "Customize Java installation" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("switch", { name: "Customize Java arguments" }),
    ).toBeInTheDocument();

    const customizeWindow = screen.getByRole("switch", {
      name: "Customize window settings",
    });
    expect(screen.queryByLabelText("Full screen")).not.toBeInTheDocument();
    fireEvent.click(customizeWindow);
    expect(screen.getByLabelText("Full screen")).toBeInTheDocument();

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        "save_instance",
        expect.objectContaining({
          folder: "Survival",
          settings: expect.objectContaining({ overrideWindow: true }),
        }),
      );
    });
  });

  it("saves an exact memory override from the numeric input", async () => {
    renderDialog();
    await screen.findByText("Survival");
    fireEvent.click(screen.getByRole("button", { name: "Override Settings" }));
    fireEvent.click(
      screen.getByRole("switch", { name: "Customize memory allocation" }),
    );
    const input = screen.getByRole("spinbutton", {
      name: "Memory Allocation (MB)",
    });
    fireEvent.change(input, { target: { value: "5001" } });
    expect(screen.getByRole("slider", { name: "Memory" })).toHaveAttribute(
      "aria-valuenow",
      "5001",
    );
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("save_instance", {
        folder: "Survival",
        settings: expect.objectContaining({
          overrideMemory: true,
          maxMemoryMb: 5001,
        }),
      }),
    );
  });
});
