import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  mockSettingsCommands,
  renderSettings,
} from "@/features/settings/fixtures/render-settings";

vi.mock("@tauri-apps/api/core", async () => {
  const { invoke: mocked } = await import(
    "@/features/settings/fixtures/invoke"
  );
  return { invoke: mocked };
});

describe("AccountSection", () => {
  beforeEach(() => {
    mockSettingsCommands();
  });

  it("shows the offline username", async () => {
    await renderSettings();

    expect(await screen.findByLabelText("Username")).toHaveValue("Steve");
    expect(screen.getByRole("heading", { name: "Account" })).toBeVisible();
    expect(
      screen.getByText(
        "Offline player name. Microsoft sign-in will live here later.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Offline name used when launching the game."),
    ).toBeInTheDocument();
  });
});
