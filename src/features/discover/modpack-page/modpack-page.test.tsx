import { focusManager } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { toast } from "sonner";
import { describe, expect, it, vi } from "vitest";

import { mockListLayout } from "@/features/discover/fixtures/list-layout";
import {
  modpackDetails,
  modpackVersions,
} from "@/features/discover/fixtures/modpacks";
import {
  classicInstance,
  invoke,
  mockContextMenuCommands,
  renderAt,
  setupInstancesPageTests,
} from "@/features/instances/tests/support/render-instances";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

vi.mock("sonner", async (importOriginal) => {
  const actual = await importOriginal<typeof import("sonner")>();
  return { ...actual, toast: { ...actual.toast, error: vi.fn() } };
});

const createdInstance = {
  ...classicInstance,
  folder: "fabulously-optimized",
  name: "Fabulously Optimized",
};

function mockInstall(
  versions: () => Promise<unknown> = () => Promise.resolve(modpackVersions()),
) {
  mockContextMenuCommands((command) => {
    if (command === "get_modpack") return Promise.resolve(modpackDetails());
    if (command === "list_modpack_versions") return versions();
    if (command === "create_instance_from_modpack") {
      return Promise.resolve(createdInstance);
    }
    return undefined;
  });
}

async function clickInstall() {
  const button = await screen.findByRole("button", { name: "Install" });
  await waitFor(() => expect(button).toBeEnabled());
  fireEvent.click(button);
}

function installedPack(projectId = "1KVo5zza"): InstanceEntry {
  return {
    ...classicInstance,
    name: "Fabulously Optimized",
    installation: {
      state: "ready",
      source: {
        type: "modpack",
        source: "modrinth",
        projectId,
        versionId: "v1",
        iconUrl: null,
      },
    },
  };
}

function mockInstalledPack(projectId = "1KVo5zza") {
  mockContextMenuCommands((command) => {
    if (command === "get_modpack") return Promise.resolve(modpackDetails());
    if (command === "list_modpack_versions") {
      return Promise.resolve(modpackVersions());
    }
    if (command === "list_instances") {
      return Promise.resolve([installedPack(projectId)]);
    }
    if (command === "create_instance_from_modpack") {
      return Promise.resolve(createdInstance);
    }
    return undefined;
  });
}

function mockModpack(details: () => Promise<unknown>) {
  mockContextMenuCommands((command) => {
    if (command === "get_modpack") return details();
    if (command === "list_modpack_versions") {
      return Promise.resolve(modpackVersions());
    }
    if (command === "open_external_url") return Promise.resolve();
    return undefined;
  });
}

describe("Modpack page", () => {
  setupInstancesPageTests();

  it("shows the pack's header, description and sidebar", async () => {
    mockModpack(() => Promise.resolve(modpackDetails()));
    await renderAt("/discover/modrinth/1KVo5zza");

    expect(
      await screen.findByRole("heading", {
        level: 1,
        name: "Fabulously Optimized",
      }),
    ).toBeInTheDocument();
    expect(screen.getByText("by robotkoer")).toBeInTheDocument();
    expect(screen.getByText("19.2M downloads")).toBeInTheDocument();
    expect(screen.getByText("5.1K followers")).toBeInTheDocument();
    expect(screen.getByText(/^Updated /, { selector: "time" })).toHaveAttribute(
      "title",
      "Nov 1, 2026",
    );
    const categories = screen.getByRole("list", { name: "Categories" });
    expect(within(categories).getAllByRole("listitem")).toHaveLength(2);
    expect(
      within(screen.getByRole("tabpanel")).getByRole("heading", {
        name: "Features",
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("complementary", { name: "About this modpack" }),
    ).toHaveTextContent("Compatibility");
    expect(invoke).toHaveBeenCalledWith("get_modpack", {
      source: "modrinth",
      projectId: "1KVo5zza",
    });
  });

  it("counts versions on their tab and shows the gallery on its own", async () => {
    mockModpack(() => Promise.resolve(modpackDetails()));
    await renderAt("/discover/modrinth/1KVo5zza");

    expect(
      await screen.findByRole("tab", { name: "Versions 2" }),
    ).toBeInTheDocument();
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Gallery" }));

    const gallery = await screen.findByRole("list", { name: "Gallery" });
    expect(
      within(gallery).getByRole("img", { name: "Title screen" }),
    ).toHaveAttribute(
      "src",
      "https://cdn.modrinth.com/data/1KVo5zza/images/title.webp",
    );
  });

  it("has no gallery tab when the pack has no images", async () => {
    mockModpack(() => Promise.resolve(modpackDetails({ gallery: [] })));
    await renderAt("/discover/modrinth/1KVo5zza");

    await screen.findByRole("tab", { name: "Description" });
    expect(
      screen.queryByRole("tab", { name: "Gallery" }),
    ).not.toBeInTheDocument();
  });

  it("opens the project page in the system browser", async () => {
    mockModpack(() => Promise.resolve(modpackDetails()));
    await renderAt("/discover/modrinth/1KVo5zza");

    fireEvent.click(
      await screen.findByRole("button", { name: "Open on Modrinth" }),
    );

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("open_external_url", {
        url: "https://modrinth.com/modpack/fabulously-optimized",
      }),
    );
  });

  it("does not reload the pack or its versions when the window regains focus", async () => {
    mockModpack(() => Promise.resolve(modpackDetails()));
    await renderAt("/discover/modrinth/1KVo5zza");
    await screen.findByRole("tab", { name: /Versions 2/ });
    const calls = () =>
      invoke.mock.calls.filter(
        ([command]) =>
          command === "get_modpack" || command === "list_modpack_versions",
      ).length;
    const before = calls();

    act(() => {
      focusManager.setFocused(false);
      focusManager.setFocused(true);
    });

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(calls()).toBe(before);
  });

  it("installs the newest release under the pack's name without a dialog", async () => {
    mockInstall();
    const { router } = await renderAt("/discover/modrinth/1KVo5zza");
    await clickInstall();

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("create_instance_from_modpack", {
        name: "Fabulously Optimized",
        source: "modrinth",
        projectId: "1KVo5zza",
        versionId: "v7",
        iconUrl: "https://cdn.modrinth.com/data/1KVo5zza/icon.webp",
      }),
    );
    await waitFor(() =>
      expect(router.state.location.search).toEqual({
        prepare: "fabulously-optimized",
      }),
    );
    expect(router.state.location.pathname).toBe("/");
  });

  it("installs the newest version even when it is a beta", async () => {
    const [release, older] = modpackVersions();
    mockInstall(() =>
      Promise.resolve([
        {
          ...older,
          id: "v8-beta",
          name: "8.0.0-beta",
          versionType: "beta",
        },
        release,
        older,
      ]),
    );
    await renderAt("/discover/modrinth/1KVo5zza");
    await clickInstall();

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "create_instance_from_modpack",
        expect.objectContaining({ versionId: "v8-beta" }),
      ),
    );
  });

  it("installs the version chosen on the Versions tab", async () => {
    mockListLayout();
    mockInstall();
    await renderAt("/discover/modrinth/1KVo5zza");

    fireEvent.mouseDown(await screen.findByRole("tab", { name: /Versions/ }));
    fireEvent.click(
      await screen.findByRole("button", { name: "Install 6.4.0" }),
    );

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "create_instance_from_modpack",
        expect.objectContaining({
          name: "Fabulously Optimized",
          versionId: "v6",
        }),
      ),
    );
  });

  it("numbers the instance when the pack name is already used", async () => {
    mockContextMenuCommands((command) => {
      if (command === "get_modpack") return Promise.resolve(modpackDetails());
      if (command === "list_modpack_versions") {
        return Promise.resolve(modpackVersions());
      }
      if (command === "list_instances") {
        return Promise.resolve([
          { ...classicInstance, name: "Fabulously Optimized" },
        ]);
      }
      if (command === "create_instance_from_modpack") {
        return Promise.resolve(createdInstance);
      }
      return undefined;
    });
    await renderAt("/discover/modrinth/1KVo5zza");
    await clickInstall();

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "create_instance_from_modpack",
        expect.objectContaining({ name: "Fabulously Optimized (1)" }),
      ),
    );
  });

  it("asks before installing a modpack that is already in the library", async () => {
    mockInstalledPack();
    await renderAt("/discover/modrinth/1KVo5zza");
    await clickInstall();

    const dialog = await screen.findByRole("dialog", {
      name: "Install this modpack again?",
    });
    expect(dialog).toHaveTextContent(
      "An instance of this modpack already exists. Are you sure you want to install it?",
    );
    expect(invoke).not.toHaveBeenCalledWith(
      "create_instance_from_modpack",
      expect.anything(),
    );

    fireEvent.click(within(dialog).getByRole("button", { name: "Install" }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "create_instance_from_modpack",
        expect.objectContaining({
          name: "Fabulously Optimized (1)",
          versionId: "v7",
        }),
      ),
    );
  });

  it("does not install when the user cancels the existing-instance question", async () => {
    mockInstalledPack();
    await renderAt("/discover/modrinth/1KVo5zza");
    await clickInstall();

    const dialog = await screen.findByRole("dialog", {
      name: "Install this modpack again?",
    });
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));

    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    expect(invoke).not.toHaveBeenCalledWith(
      "create_instance_from_modpack",
      expect.anything(),
    );
  });

  it("asks again when an older version is installed for a pack already in the library", async () => {
    mockListLayout();
    mockInstalledPack();
    await renderAt("/discover/modrinth/1KVo5zza");

    fireEvent.mouseDown(await screen.findByRole("tab", { name: /Versions/ }));
    fireEvent.click(
      await screen.findByRole("button", { name: "Install 6.4.0" }),
    );
    const dialog = await screen.findByRole("dialog", {
      name: "Install this modpack again?",
    });
    fireEvent.click(within(dialog).getByRole("button", { name: "Install" }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "create_instance_from_modpack",
        expect.objectContaining({ versionId: "v6" }),
      ),
    );
  });

  it("explains when the pack has no versions to install", async () => {
    vi.mocked(toast.error).mockClear();
    mockInstall(() => Promise.resolve([]));
    await renderAt("/discover/modrinth/1KVo5zza");
    await clickInstall();

    expect(toast.error).toHaveBeenCalledWith(
      "This pack has no versions to install.",
    );
    expect(invoke).not.toHaveBeenCalledWith(
      "create_instance_from_modpack",
      expect.anything(),
    );
  });

  it("explains when versions could not be listed", async () => {
    vi.mocked(toast.error).mockClear();
    mockInstall(() => Promise.reject("HTTP request failed"));
    await renderAt("/discover/modrinth/1KVo5zza");
    await clickInstall();

    expect(toast.error).toHaveBeenCalledWith(
      "The network request failed. Check your connection and try again.",
    );
    expect(invoke).not.toHaveBeenCalledWith(
      "create_instance_from_modpack",
      expect.anything(),
    );
  });

  it("installs the pack on screen after moving to another pack", async () => {
    mockContextMenuCommands((command, args) => {
      if (command === "get_modpack") {
        const projectId =
          args !== null && typeof args === "object" && "projectId" in args
            ? String(args.projectId)
            : "";
        return Promise.resolve(
          modpackDetails({
            id: projectId,
            name: projectId === "other" ? "Other Pack" : "Fabulously Optimized",
          }),
        );
      }
      if (command === "list_modpack_versions") {
        return Promise.resolve(modpackVersions());
      }
      if (command === "create_instance_from_modpack") {
        return Promise.resolve(createdInstance);
      }
      return undefined;
    });
    const { router } = await renderAt("/discover/modrinth/other");
    await screen.findByRole("heading", { level: 1, name: "Other Pack" });
    await router.navigate({
      to: "/discover/$source/$projectId",
      params: { source: "modrinth", projectId: "1KVo5zza" },
    });
    await clickInstall();

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "create_instance_from_modpack",
        expect.objectContaining({
          name: "Fabulously Optimized",
          projectId: "1KVo5zza",
        }),
      ),
    );
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("explains when the pack cannot be loaded", async () => {
    mockModpack(() => Promise.reject("HTTP request failed"));
    await renderAt("/discover/modrinth/1KVo5zza");

    expect(
      await screen.findByRole("status", {
        name: "Could not load this modpack",
      }),
    ).toBeInTheDocument();
  });

  it("rejects links with an unknown source without calling the backend", async () => {
    mockModpack(() => Promise.resolve(modpackDetails()));
    await renderAt("/discover/ftb/123");

    expect(
      await screen.findByRole("status", { name: "Modpack not found" }),
    ).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith("get_modpack", expect.anything());
  });
});
