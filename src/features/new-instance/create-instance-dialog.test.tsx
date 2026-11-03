import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  createdInstance,
  invoke,
  mockCreateCommands,
  mockVersionViewport,
  openCreateDialog,
  openCreateMenu,
  openDialog,
  renderApp,
} from "@/features/new-instance/fixtures/render-create-dialog";
import type { LoaderBuild } from "@/lib/generated/LoaderBuild";

async function openDetails() {
  const dialog = await openCreateDialog();
  await screen.findByLabelText("Instance Name");
  await waitFor(() =>
    expect(screen.getByRole("combobox", { name: "Game version" })).toHaveValue(
      "1.20.1",
    ),
  );
  return dialog;
}

async function chooseUpload() {
  const menu = await openCreateMenu();
  fireEvent.click(
    within(menu).getByRole("menuitem", { name: "Upload a pack file" }),
  );
}

async function typeName(name: string) {
  const field = await screen.findByLabelText("Instance Name");
  fireEvent.change(field, { target: { value: name } });
  await waitFor(() => expect(field).toHaveValue(name));
}

async function submit() {
  const button = screen.getByRole("button", { name: "Create" });
  await waitFor(() => expect(button).toBeEnabled());
  fireEvent.click(button);
}

describe("CreateInstanceDialog", () => {
  beforeEach(() => {
    mockCreateCommands();
    mockVersionViewport();
    openDialog.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("offers scratch, modpack and upload in a menu, with no popup yet", async () => {
    await renderApp();
    const menu = await openCreateMenu();

    expect(
      within(menu)
        .getAllByRole("menuitem")
        .map((item) => item.textContent),
    ).toEqual(["From scratch", "From a modpack", "Upload a pack file"]);
    expect(
      within(menu).getByRole("menuitem", { name: "From a modpack" }),
    ).toHaveAttribute("aria-disabled", "true");
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("introduces the dialog with a title and no icon", async () => {
    await renderApp();
    const dialog = await openCreateDialog();

    expect(dialog.querySelector("[data-slot='create-header-icon']")).toBeNull();
    expect(
      within(dialog).getByRole("heading", { name: "Create instance" }),
    ).toBeVisible();
  });

  it("shows the header without a divider on the same background as the content", async () => {
    await renderApp();
    const dialog = await openCreateDialog();

    const header = dialog.querySelector("[data-slot='dialog-header']");
    expect(header).toHaveClass("-mx-4");
    expect(header).not.toHaveClass("border-b");
    expect(header).not.toHaveClass("bg-muted/50");
    expect(header).not.toHaveClass("border-border/40");
  });

  it("uses less side padding and no tinted footer", async () => {
    await renderApp();
    const dialog = await openDetails();

    expect(dialog).toHaveClass("px-4", "py-6");
    expect(dialog).not.toHaveClass("p-6");
    const footer = dialog.querySelector("[data-slot='dialog-footer']");
    expect(footer).toHaveClass("-mx-4", "-mb-6", "px-4", "bg-transparent");
    expect(footer).not.toHaveClass("bg-muted/50");
  });

  it("closes with the same corner button as the other dialogs", async () => {
    await renderApp();
    const dialog = await openCreateDialog();

    const closeButtons = within(dialog).getAllByRole("button", {
      name: "Close",
    });
    expect(closeButtons).toHaveLength(1);
    const close = closeButtons[0] as HTMLElement;
    expect(close).toHaveAttribute("data-slot", "dialog-close");
    expect(close).toHaveClass("absolute", "top-2", "right-2");
    expect(
      dialog.querySelector("[data-slot='dialog-header']")?.contains(close),
    ).toBe(false);
    fireEvent.click(close);
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("creates a vanilla instance with the newest version preselected", async () => {
    await renderApp();
    await openDetails();

    const version = screen.getByRole("combobox", { name: "Game version" });
    await typeName("Survival");
    fireEvent.focus(version);
    fireEvent.change(version, { target: { value: "1.19" } });
    fireEvent.click(screen.getByRole("option", { name: "1.19.4" }));
    expect(version).toHaveValue("1.19.4");
    await submit();

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("create_instance_with_platform", {
        name: "Survival",
        platform: "vanilla",
        gameVersion: "1.19.4",
        loaderChannel: "stable",
        loaderVersion: null,
      }),
    );
    expect(
      await screen.findByRole("heading", { name: "Survival" }),
    ).toBeVisible();
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("creates with the default name when the field is left empty", async () => {
    await renderApp();
    await openDetails();

    await submit();

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("create_instance_with_platform", {
        name: "Vanilla 1.20.1",
        platform: "vanilla",
        gameVersion: "1.20.1",
        loaderChannel: "stable",
        loaderVersion: null,
      }),
    );
  });

  it("numbers the default name when an instance already has it", async () => {
    mockCreateCommands(undefined, [
      createdInstance("Vanilla 1.20.1"),
      createdInstance("Vanilla 1.20.1 (1)"),
    ]);
    await renderApp();
    await openDetails();

    expect(screen.getByLabelText("Instance Name")).toHaveAttribute(
      "placeholder",
      "Vanilla 1.20.1 (2)",
    );
    await submit();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("create_instance_with_platform", {
        name: "Vanilla 1.20.1 (2)",
        platform: "vanilla",
        gameVersion: "1.20.1",
        loaderChannel: "stable",
        loaderVersion: null,
      }),
    );
  });

  it("accepts a name that only differs in punctuation", async () => {
    mockCreateCommands(undefined, [createdInstance("Survival")]);
    await renderApp();
    await openDetails();

    await typeName("Survival!");

    expect(
      screen.queryByText("An instance with this name already exists."),
    ).toBeNull();
    expect(screen.getByRole("button", { name: "Create" })).toBeEnabled();
  });

  it("refuses a typed name that another instance already has", async () => {
    mockCreateCommands(undefined, [createdInstance("Survival")]);
    await renderApp();
    await openDetails();

    await typeName("survival");
    expect(
      await screen.findByText("An instance with this name already exists."),
    ).toBeVisible();
    expect(screen.getByRole("button", { name: "Create" })).toBeDisabled();
    expect(screen.getByLabelText("Instance Name")).toBeInvalid();
    await typeName("Survival 2");
    expect(
      screen.queryByText("An instance with this name already exists."),
    ).toBeNull();
    expect(screen.getByRole("button", { name: "Create" })).toBeEnabled();
  });

  it("numbers an uploaded pack whose name is already taken", async () => {
    openDialog.mockResolvedValueOnce("C:\\packs\\Cobble Pack.mrpack");
    mockCreateCommands(undefined, [createdInstance("Cobble Pack")]);
    await renderApp();
    await chooseUpload();

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("create_instance_from_modpack_file", {
        name: "Cobble Pack (1)",
        path: "C:\\packs\\Cobble Pack.mrpack",
      }),
    );
  });

  it("does not let a loader without builds be created", async () => {
    mockCreateCommands((command) => {
      if (command === "list_loader_builds") {
        return [];
      }
    });
    await renderApp();
    await openDetails();

    fireEvent.click(screen.getByRole("button", { name: /^Forge/ }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Latest" })).toBeDisabled(),
    );
    expect(screen.getByRole("button", { name: "Stable" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Other" })).toBeDisabled();
    expect(screen.queryByText(/has no build/)).toBeNull();
    expect(screen.getByRole("button", { name: "Create" })).toBeDisabled();
  });

  it("creates with the latest channel when the loader has no stable build", async () => {
    mockCreateCommands((command) => {
      if (command === "list_loader_builds") {
        return [{ version: "0.31.0-beta.5", stable: false }];
      }
    });
    await renderApp();
    await openDetails();

    fireEvent.click(screen.getByRole("button", { name: /^Quilt/ }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Latest" })).toHaveAttribute(
        "aria-pressed",
        "true",
      ),
    );
    expect(screen.getByRole("button", { name: "Stable" })).toBeDisabled();
    expect(screen.queryByText(/No stable/)).toBeNull();
    await submit();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("create_instance_with_platform", {
        name: "Quilt 1.20.1",
        platform: "quilt",
        gameVersion: "1.20.1",
        loaderChannel: "latest",
        loaderVersion: null,
      }),
    );
  });

  it("blocks creating while the loader builds are still loading", async () => {
    let finish: (builds: LoaderBuild[]) => void = () => {};
    const pending = new Promise<LoaderBuild[]>((resolve) => {
      finish = resolve;
    });
    mockCreateCommands((command) => {
      if (command === "list_loader_builds") {
        return pending;
      }
    });
    await renderApp();
    await openDetails();

    fireEvent.click(screen.getByRole("button", { name: /^Fabric/ }));
    await screen.findByRole("group", { name: "Loader channel" });
    expect(screen.queryByText(/Checking/)).toBeNull();
    expect(screen.getByRole("button", { name: "Create" })).toBeDisabled();
    finish([{ version: "0.16.0", stable: true }]);
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Create" })).toBeEnabled(),
    );
  });

  it("keeps the form inside the scroll area clear of clipped outlines", async () => {
    await renderApp();
    const dialog = await openDetails();

    const scroll = dialog.querySelector("[data-slot='create-scroll']");
    expect(scroll).toHaveClass("-mx-1", "px-1");
  });

  it("keeps the header and the actions out of the scrolling area", async () => {
    await renderApp();
    const dialog = await openDetails();

    const scroll = dialog.querySelector("[data-slot='create-scroll']");
    expect(scroll).not.toBeNull();
    expect(scroll?.contains(screen.getByLabelText("Instance Name"))).toBe(true);
    expect(
      scroll?.contains(screen.getByRole("button", { name: "Create" })),
    ).toBe(false);
    expect(
      scroll?.contains(screen.getByRole("button", { name: "Cancel" })),
    ).toBe(false);
    expect(
      scroll?.contains(
        within(dialog).getByRole("heading", { name: "Create instance" }),
      ),
    ).toBe(false);
  });

  it("lets the channel be changed with the cards and sends it", async () => {
    await renderApp();
    await openDetails();
    fireEvent.click(screen.getByRole("button", { name: /^Fabric/ }));
    await screen.findByRole("group", { name: "Loader channel" });

    fireEvent.click(screen.getByRole("button", { name: "Latest" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Latest" })).toHaveAttribute(
        "aria-pressed",
        "true",
      ),
    );
    await submit();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("create_instance_with_platform", {
        name: "Fabric 1.20.1",
        platform: "fabric",
        gameVersion: "1.20.1",
        loaderChannel: "latest",
        loaderVersion: null,
      }),
    );
  });

  it("lets Other pick a loader version by hand", async () => {
    await renderApp();
    await openDetails();
    fireEvent.click(screen.getByRole("button", { name: /^Fabric/ }));
    await screen.findByRole("group", { name: "Loader channel" });

    fireEvent.click(screen.getByRole("button", { name: "Other" }));
    expect(
      await screen.findByRole("combobox", { name: "Loader version" }),
    ).toBeVisible();
    await submit();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("create_instance_with_platform", {
        name: "Fabric 1.20.1",
        platform: "fabric",
        gameVersion: "1.20.1",
        loaderChannel: "manual",
        loaderVersion: "0.16.0",
      }),
    );
  });

  it("asks for a loader channel on Fabric and sends it", async () => {
    await renderApp();
    await openDetails();

    expect(screen.queryByRole("group", { name: "Loader channel" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: /^Fabric/ }));
    expect(
      await screen.findByRole("group", { name: "Loader channel" }),
    ).toBeVisible();
    expect(screen.getByRole("button", { name: "Stable" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await typeName("Modded");
    await submit();

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("create_instance_with_platform", {
        name: "Modded",
        platform: "fabric",
        gameVersion: "1.20.1",
        loaderChannel: "stable",
        loaderVersion: null,
      }),
    );
  });

  it("includes snapshots when asked", async () => {
    await renderApp();
    await openDetails();

    const version = screen.getByRole("combobox", { name: "Game version" });
    fireEvent.focus(version);
    fireEvent.change(version, { target: { value: "24w" } });
    expect(screen.queryByRole("option", { name: "24w14a" })).toBeNull();
    fireEvent.click(screen.getByRole("switch", { name: "Include snapshots" }));
    fireEvent.focus(version);
    fireEvent.change(version, { target: { value: "24w" } });
    expect(await screen.findByRole("option", { name: "24w14a" })).toBeVisible();
  });

  it("cancels without creating anything", async () => {
    await renderApp();
    await openDetails();

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(
      invoke.mock.calls.some(
        ([command]) => command === "create_instance_with_platform",
      ),
    ).toBe(false);
  });

  it("uploads a pack file through the system picker", async () => {
    openDialog.mockResolvedValueOnce("C:\\packs\\Cobble Pack.mrpack");
    await renderApp();
    await chooseUpload();

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("create_instance_from_modpack_file", {
        name: "Cobble Pack",
        path: "C:\\packs\\Cobble Pack.mrpack",
      }),
    );
    expect(
      await screen.findByRole("heading", { name: "Cobble Pack" }),
    ).toBeVisible();
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("does nothing when the picker is cancelled", async () => {
    openDialog.mockResolvedValueOnce(null);
    await renderApp();
    await chooseUpload();

    await waitFor(() => expect(openDialog).toHaveBeenCalledOnce());
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(
      invoke.mock.calls.some(
        ([command]) => command === "create_instance_from_modpack_file",
      ),
    ).toBe(false);
  });

  it("reports an upload that the backend rejects", async () => {
    openDialog.mockResolvedValueOnce("C:\\packs\\broken.zip");
    mockCreateCommands((command) => {
      if (command === "create_instance_from_modpack_file") {
        return Promise.reject(new Error("modpack archive is invalid"));
      }
    });
    await renderApp();
    await chooseUpload();

    expect(await screen.findByText(/archive could not be read/i)).toBeVisible();
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});
