import { fireEvent, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";

import {
  mockWizardCommands,
  renderWizard,
  selectSource,
  wizardVersions,
} from "@/features/new-instance/fixtures/render-wizard";

describe("NewInstancePage navigation and recovery", () => {
  beforeEach(() => mockWizardCommands());

  it("starts on the scratch source with the newest release selected", async () => {
    await renderWizard();
    expect(
      await screen.findByRole("tab", { name: "From scratch" }),
    ).toHaveAttribute("aria-selected", "true");
    expect(
      await screen.findByRole("button", { name: "1.20.1" }),
    ).toHaveAttribute("aria-pressed", "true");
  });

  it("keeps the typed name when switching sources", async () => {
    await renderWizard();
    const name = await screen.findByLabelText("Instance Name");
    fireEvent.change(name, { target: { value: "Survival" } });
    await selectSource("Pack file");
    await selectSource("From scratch");
    expect(await screen.findByLabelText("Instance Name")).toHaveValue(
      "Survival",
    );
  });

  it("retries a failed version request and recovers the version picker", async () => {
    let offline = true;
    mockWizardCommands((command) => {
      if (command === "list_versions") {
        return offline
          ? Promise.reject(new Error("offline"))
          : Promise.resolve(wizardVersions);
      }
      return undefined;
    });
    await renderWizard();
    expect(
      await screen.findByText(/Could not load the version list/),
    ).toBeVisible();
    offline = false;
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(await screen.findByRole("button", { name: "1.20.1" })).toBeVisible();
  });
});
