import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";

import {
  invoke,
  mockWizardCommands,
  openDialog,
  renderWizard,
  selectChoice,
  selectSource,
  wizardInstance,
} from "@/features/new-instance/fixtures/render-wizard";

async function create() {
  const button = screen.getByRole("button", { name: "Create instance" });
  await waitFor(() => expect(button).toBeEnabled());
  fireEvent.click(button);
}

describe("NewInstancePage creation", () => {
  beforeEach(() => mockWizardCommands());

  it("keeps Create disabled until the form is complete", async () => {
    await renderWizard();
    const button = await screen.findByRole("button", {
      name: "Create instance",
    });
    expect(button).toBeDisabled();
    const name = await screen.findByLabelText("Instance Name");
    fireEvent.change(name, { target: { value: "Survival" } });
    await waitFor(() => expect(button).toBeEnabled());
  });

  it("creates a vanilla instance and returns to its library page", async () => {
    await renderWizard();
    const name = await screen.findByLabelText("Instance Name");
    fireEvent.change(name, { target: { value: "Survival" } });
    await waitFor(() => expect(name).toHaveValue("Survival"));
    await selectChoice("1.20.1");
    await create();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("create_instance_with_platform", {
        name: "Survival",
        platform: "vanilla",
        gameVersion: "1.20.1",
        loaderChannel: "stable",
        loaderVersion: null,
      }),
    );
    expect(
      await screen.findByRole("heading", { name: "Survival" }),
    ).toBeVisible();
  });

  it("creates a Fabric instance with the stable loader channel", async () => {
    await renderWizard();
    await selectChoice(/^Fabric/);
    const name = await screen.findByLabelText("Instance Name");
    fireEvent.change(name, { target: { value: "Modded" } });
    await waitFor(() => expect(name).toHaveValue("Modded"));
    expect(
      await screen.findByRole("combobox", { name: "Loader channel" }),
    ).toBeVisible();
    await create();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("create_instance_with_platform", {
        name: "Modded",
        platform: "fabric",
        gameVersion: "1.20.1",
        loaderChannel: "stable",
        loaderVersion: null,
      }),
    );
    expect(
      await screen.findByRole("heading", { name: "Modded" }),
    ).toBeVisible();
  });

  it("imports a Modrinth modpack and returns to its library page", async () => {
    await renderWizard();
    await selectSource("Modpack");
    const search = await screen.findByLabelText("Search modpacks");
    fireEvent.change(search, { target: { value: "cobble" } });
    await waitFor(() => expect(search).toHaveValue("cobble"));
    fireEvent.click(await screen.findByRole("button", { name: /Cobblemon/ }));
    await waitFor(() =>
      expect(screen.getByLabelText(/^Name$/)).toHaveValue("Cobblemon"),
    );
    await screen.findByRole("button", { name: /1\.0\.0/ });
    await create();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("create_instance_from_modpack", {
        name: "Cobblemon",
        source: "modrinth",
        projectId: "ABC",
        versionId: "v1",
        iconUrl: null,
      }),
    );
    expect(
      await screen.findByRole("heading", { name: "Cobblemon" }),
    ).toBeVisible();
  });

  it("imports a local pack file and returns to its library page", async () => {
    openDialog.mockResolvedValueOnce("C:\\packs\\Cobble Pack.mrpack");
    await renderWizard();
    await selectSource("Pack file");
    fireEvent.click(await screen.findByRole("button", { name: "Choose file" }));
    await waitFor(() =>
      expect(screen.getByLabelText(/^Name$/)).toHaveValue("Cobble Pack"),
    );
    await create();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("create_instance_from_modpack_file", {
        name: "Cobble Pack",
        path: "C:\\packs\\Cobble Pack.mrpack",
      }),
    );
    expect(
      await screen.findByRole("heading", { name: "Cobble Pack" }),
    ).toBeVisible();
  });

  it("keeps the file source empty when the picker is cancelled", async () => {
    openDialog.mockResolvedValueOnce(null);
    await renderWizard();
    await selectSource("Pack file");
    fireEvent.click(await screen.findByRole("button", { name: "Choose file" }));
    await waitFor(() => expect(openDialog).toHaveBeenCalled());
    expect(screen.getByText("No file selected")).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Create instance" }),
    ).toBeDisabled();
  });

  it("clones an existing instance under a new name", async () => {
    mockWizardCommands(undefined, [wizardInstance("Survival")]);
    await renderWizard();
    await selectSource("Clone");
    const list = await screen.findByRole("list", { name: "Instance to clone" });
    fireEvent.click(within(list).getByRole("button", { name: /Survival/ }));
    await waitFor(() =>
      expect(screen.getByLabelText(/^Name$/)).toHaveValue("Survival copy"),
    );
    await create();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("duplicate_instance", {
        folder: "Survival",
        name: "Survival copy",
      }),
    );
    expect(
      await screen.findByRole("heading", { name: "Survival copy" }),
    ).toBeVisible();
  });
});
