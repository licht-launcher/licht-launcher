import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { StepConfirm } from "@/features/new-instance/components/step-confirm";
import { initialWizardState } from "@/features/new-instance/model/wizard";

function card(label: string) {
  return screen.getByRole("region", { name: label });
}

describe("StepConfirm", () => {
  it("summarizes a scratch instance in one card per choice", () => {
    render(
      <StepConfirm
        state={{
          ...initialWizardState,
          step: "confirm",
          source: "scratch",
          platform: "fabric",
          name: "  Modded ",
          gameVersion: "1.20.1",
          loaderChannel: "latest",
        }}
      />,
    );

    expect(within(card("Name")).getByText("Modded")).toBeVisible();
    expect(within(card("Platform")).getByText("Fabric")).toBeVisible();
    expect(within(card("Game version")).getByText("1.20.1")).toBeVisible();
    expect(within(card("Loader")).getByText("latest")).toBeVisible();
  });

  it("omits the loader card for vanilla", () => {
    render(
      <StepConfirm
        state={{
          ...initialWizardState,
          step: "confirm",
          source: "scratch",
          platform: "vanilla",
          name: "Survival",
          gameVersion: "1.20.1",
        }}
      />,
    );

    expect(screen.queryByRole("region", { name: "Loader" })).toBeNull();
  });

  it("summarizes a local pack file", () => {
    render(
      <StepConfirm
        state={{
          ...initialWizardState,
          step: "confirm",
          source: "file",
          name: "Cobble Pack",
          filePath: "C:\\packs\\Cobble Pack.mrpack",
        }}
      />,
    );

    expect(within(card("Name")).getByText("Cobble Pack")).toBeVisible();
    expect(
      within(card("File")).getByText("C:\\packs\\Cobble Pack.mrpack"),
    ).toBeVisible();
  });
});
