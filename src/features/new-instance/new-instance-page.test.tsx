import { fireEvent, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";

import {
  mockWizardCommands,
  renderWizard,
  wizardVersions,
} from "@/features/new-instance/fixtures/render-wizard";

describe("NewInstancePage navigation and recovery", () => {
  beforeEach(() => mockWizardCommands());

  it("restores progress from the URL search params", async () => {
    await renderWizard(
      "/instances/new?step=details&source=scratch&platform=vanilla&name=Survival&gameVersion=1.20.1",
    );
    expect(await screen.findByLabelText("Instance Name")).toHaveValue(
      "Survival",
    );
    expect(
      await screen.findByRole("button", { name: "1.20.1" }),
    ).toHaveAttribute("aria-pressed", "true");
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
    await renderWizard(
      "/instances/new?step=details&source=scratch&platform=vanilla&name=Survival",
    );
    expect(
      await screen.findByText(/Could not load the version list/),
    ).toBeVisible();
    offline = false;
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(await screen.findByRole("button", { name: "1.20.1" })).toBeVisible();
  });
});
