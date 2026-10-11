import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ComponentProps } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { Toaster } from "@/components/ui/sonner";
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
        onChangeVersion={() => {}}
        onOpenFolder={() => {}}
        onDuplicate={() => {}}
        onDelete={() => {}}
        {...props}
      />
      <Toaster />
    </QueryClientProvider>,
  );
}

describe("InstanceSettingsDialog", () => {
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
            dataDirectory: null,
          },
          applicationDirectory: "C:\\Licht",
          memoryTotalMb: 16384,
          memoryRecommendedMb: 4096,
          memoryMinimumMb: 512,
          memoryMaximumMb: 8192,
          memoryStepMb: 256,
        });
      }
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.20.1", versionType: "release", installed: true },
          { id: "1.21.1", versionType: "release", installed: false },
          { id: "24w14a", versionType: "snapshot", installed: false },
        ]);
      }
      return Promise.resolve(entry);
    });
  });

  it("opens on General with rename, folder, duplicate, and delete", async () => {
    const onRename = vi.fn();
    const onOpenFolder = vi.fn();
    const onDuplicate = vi.fn();
    const onDelete = vi.fn();
    const onChangeVersion = vi.fn();
    renderDialog({
      onRename,
      onOpenFolder,
      onDuplicate,
      onDelete,
      onChangeVersion,
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
        "Platform and game version this instance launches with. Only Vanilla is available for now.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("Vanilla")).toBeInTheDocument();
    expect(screen.getByText("1.20.1")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Edit base" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Open the editor to pick another Vanilla release for this instance.",
      ),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Edit base" }));
    expect(
      await screen.findByRole("heading", { name: "Edit game version" }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByLabelText("Game version"));
    expect(
      await screen.findByRole("option", { name: "1.21.1 (will install)" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("option", { name: "24w14a (will install)" }),
    ).not.toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("option", { name: "1.21.1 (will install)" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(onChangeVersion).toHaveBeenCalledWith("1.21.1");
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
});
