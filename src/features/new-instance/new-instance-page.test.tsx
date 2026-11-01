import { fireEvent, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";

import {
  continueWizard,
  mockWizardCommands,
  renderWizard,
  selectChoice,
  wizardVersions,
} from "@/features/new-instance/fixtures/render-wizard";

describe("NewInstancePage navigation and recovery", () => {
  beforeEach(() => mockWizardCommands());

  it("preselects the scratch source so the user can continue right away", async () => {
    await renderWizard();
    expect(
      await screen.findByRole("button", { name: /From scratch/ }),
    ).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "Continue" })).toBeEnabled();
  });

  it("goes back to a finished step from the stepper and keeps the choices", async () => {
    await renderWizard();
    await continueWizard();
    await selectChoice(/^Fabric/);
    await continueWizard();
    await screen.findByLabelText("Instance Name");
    fireEvent.click(screen.getByRole("button", { name: /Platform/ }));
    expect(
      await screen.findByRole("button", { name: /^Fabric/ }),
    ).toHaveAttribute("aria-pressed", "true");
    const progress = screen.getByRole("list", { name: "Wizard progress" });
    expect(
      within(progress).getByText("Platform").closest("[aria-current]"),
    ).toHaveAttribute("aria-current", "step");
  });

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
