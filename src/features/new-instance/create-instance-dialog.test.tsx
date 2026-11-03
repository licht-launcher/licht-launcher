import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  invoke,
  mockCreateCommands,
  mockVersionViewport,
  openCreateDialog,
  openDialog,
  renderApp,
} from "@/features/new-instance/fixtures/render-create-dialog";

async function openDetails() {
  const dialog = await openCreateDialog();
  fireEvent.click(within(dialog).getByRole("button", { name: /From scratch/ }));
  await screen.findByLabelText("Instance Name");
  await waitFor(() =>
    expect(screen.getByRole("combobox", { name: "Game version" })).toHaveValue(
      "1.20.1",
    ),
  );
  return dialog;
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

  it("offers the three sources and marks modpack as coming soon", async () => {
    await renderApp();
    const dialog = await openCreateDialog();

    expect(
      within(dialog).getByRole("button", { name: /From scratch/ }),
    ).toBeEnabled();
    expect(
      within(dialog).getByRole("button", { name: /Upload a pack file/ }),
    ).toBeEnabled();
    const modpack = within(dialog).getByRole("button", {
      name: /From a modpack/,
    });
    expect(modpack).toBeDisabled();
    expect(within(modpack).getByText("Soon")).toBeVisible();
  });

  it("introduces the dialog with an icon, a title and a description", async () => {
    await renderApp();
    const dialog = await openCreateDialog();

    expect(
      dialog.querySelector("[data-slot='create-header-icon']"),
    ).not.toBeNull();
    expect(
      within(dialog).getByRole("heading", { name: "Create instance" }),
    ).toBeVisible();
    expect(within(dialog).getByText("How do you want to start?")).toBeVisible();
    fireEvent.click(
      within(dialog).getByRole("button", { name: /From scratch/ }),
    );
    expect(
      await within(dialog).findByText(
        "Name it, pick a platform and a game version.",
      ),
    ).toBeVisible();
  });

  it("keeps the close button on the same row as the title", async () => {
    await renderApp();
    const dialog = await openCreateDialog();

    const closeButtons = within(dialog).getAllByRole("button", {
      name: "Close",
    });
    expect(closeButtons).toHaveLength(1);
    const title = within(dialog).getByRole("heading", {
      name: "Create instance",
    });
    expect(closeButtons[0]?.parentElement).toBe(title.parentElement);
    fireEvent.click(closeButtons[0] as HTMLElement);
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("slides between the steps in the direction of travel", async () => {
    await renderApp();
    const dialog = await openCreateDialog();
    const direction = () =>
      dialog
        .querySelector("[data-slot='page-transition']")
        ?.getAttribute("data-direction");

    fireEvent.click(
      within(dialog).getByRole("button", { name: /From scratch/ }),
    );
    await screen.findByLabelText("Instance Name");
    expect(direction()).toBe("left");
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    await within(dialog).findByRole("button", { name: /From scratch/ });
    expect(direction()).toBe("right");
  });

  it("opens from the welcome screen as well", async () => {
    await renderApp();
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Create instance",
        description: "",
      }),
    );
    expect(
      await screen.findByRole("dialog", { name: "Create instance" }),
    ).toBeVisible();
  });

  it("creates a vanilla instance with the newest version preselected", async () => {
    await renderApp();
    await openDetails();

    const version = screen.getByRole("combobox", { name: "Game version" });
    expect(screen.getByRole("button", { name: "Create" })).toBeDisabled();
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

  it("asks for a loader channel on Fabric and sends it", async () => {
    await renderApp();
    await openDetails();

    expect(
      screen.queryByRole("combobox", { name: "Loader channel" }),
    ).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: /^Fabric/ }));
    expect(
      await screen.findByRole("combobox", { name: "Loader channel" }),
    ).toBeVisible();
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

  it("goes back to the sources without losing the dialog", async () => {
    await renderApp();
    const dialog = await openDetails();

    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(
      await within(dialog).findByRole("button", { name: /From scratch/ }),
    ).toBeVisible();
    expect(screen.queryByLabelText("Instance Name")).toBeNull();
  });

  it("uploads a pack file through the system picker", async () => {
    openDialog.mockResolvedValueOnce("C:\\packs\\Cobble Pack.mrpack");
    await renderApp();
    const dialog = await openCreateDialog();
    fireEvent.click(
      within(dialog).getByRole("button", { name: /Upload a pack file/ }),
    );

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

  it("stays on the sources when the picker is cancelled", async () => {
    openDialog.mockResolvedValueOnce(null);
    await renderApp();
    const dialog = await openCreateDialog();
    fireEvent.click(
      within(dialog).getByRole("button", { name: /Upload a pack file/ }),
    );

    await waitFor(() => expect(openDialog).toHaveBeenCalledOnce());
    expect(
      within(dialog).getByRole("button", { name: /Upload a pack file/ }),
    ).toBeEnabled();
    expect(
      invoke.mock.calls.some(
        ([command]) => command === "create_instance_from_modpack_file",
      ),
    ).toBe(false);
  });

  it("reports an upload that the backend rejects and keeps the dialog open", async () => {
    openDialog.mockResolvedValueOnce("C:\\packs\\broken.zip");
    mockCreateCommands((command) => {
      if (command === "create_instance_from_modpack_file") {
        return Promise.reject(new Error("modpack archive is invalid"));
      }
    });
    await renderApp();
    const dialog = await openCreateDialog();
    fireEvent.click(
      within(dialog).getByRole("button", { name: /Upload a pack file/ }),
    );

    expect(await screen.findByText(/archive could not be read/i)).toBeVisible();
    expect(
      screen.getByRole("dialog", { name: "Create instance" }),
    ).toBeVisible();
  });
});
