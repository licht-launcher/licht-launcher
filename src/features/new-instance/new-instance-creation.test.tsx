import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";

import {
  continueWizard,
  invoke,
  mockWizardCommands,
  renderWizard,
  selectChoice,
} from "@/features/new-instance/fixtures/render-wizard";

describe("NewInstancePage creation", () => {
  beforeEach(() => mockWizardCommands());

  it("creates a vanilla instance through the wizard and returns to its library page", async () => {
    await renderWizard();
    await selectChoice(/From scratch/);
    await continueWizard();
    await selectChoice(/^Vanilla/);
    await continueWizard();
    const name = await screen.findByLabelText("Instance Name");
    fireEvent.change(name, { target: { value: "Survival" } });
    await waitFor(() => expect(name).toHaveValue("Survival"));
    await selectChoice("1.20.1");
    await continueWizard();
    fireEvent.click(
      await screen.findByRole("button", { name: "Create instance" }),
    );
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
    await renderWizard(
      "/instances/new?step=details&source=scratch&platform=fabric&name=Modded&gameVersion=1.20.1",
    );
    expect(
      await screen.findByRole("combobox", { name: "Loader channel" }),
    ).toBeVisible();
    await continueWizard();
    fireEvent.click(
      await screen.findByRole("button", { name: "Create instance" }),
    );
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
    await selectChoice(/From a modpack/);
    await continueWizard();
    const search = await screen.findByLabelText("Search modpacks");
    fireEvent.change(search, { target: { value: "cobble" } });
    await waitFor(() => expect(search).toHaveValue("cobble"));
    fireEvent.click(await screen.findByRole("button", { name: /Cobblemon/ }));
    await continueWizard();
    expect(await screen.findByLabelText(/^Name$/)).toHaveValue("Cobblemon");
    await continueWizard();
    fireEvent.click(
      await screen.findByRole("button", { name: "Create instance" }),
    );
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
});
