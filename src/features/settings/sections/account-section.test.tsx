import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  invoke,
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
    mockSettingsCommands((command) => {
      if (command === "list_microsoft_accounts") {
        return Promise.resolve([]);
      }
      if (command === "active_microsoft_account") {
        return Promise.resolve(null);
      }
      return undefined;
    });
  });

  it("shows offline username when no Microsoft account is active", async () => {
    await renderSettings();

    expect(await screen.findByLabelText("Offline username")).toHaveValue(
      "Steve",
    );
    expect(screen.getByRole("heading", { name: "Account" })).toBeVisible();
    expect(
      screen.getByText("Microsoft sign-in and offline player name."),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Sign in" })).toBeInTheDocument();
  });

  it("starts Microsoft browser sign-in", async () => {
    const profile = {
      uuid: "abcdef0123456789abcdef0123456789",
      name: "PlayerOne",
    };
    let signedIn = false;
    mockSettingsCommands((command) => {
      if (command === "list_microsoft_accounts") {
        return Promise.resolve(signedIn ? [profile] : []);
      }
      if (command === "active_microsoft_account") {
        return Promise.resolve(signedIn ? profile : null);
      }
      if (command === "begin_microsoft_login") {
        signedIn = true;
        return Promise.resolve(profile);
      }
      return undefined;
    });

    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Sign in" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("begin_microsoft_login");
    });
    expect(await screen.findByText("PlayerOne")).toBeInTheDocument();
    expect(screen.getByText("Active")).toBeInTheDocument();
  });
});
