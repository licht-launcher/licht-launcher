import { fireEvent, screen, waitFor, within } from "@testing-library/react";
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

  it("shows Microsoft sign-in without the developer-only offline username", async () => {
    await renderSettings();

    expect(
      await screen.findByRole("heading", { name: "Account" }),
    ).toBeVisible();
    expect(
      screen.getByRole("heading", { name: "Sign in with Microsoft", level: 4 }),
    ).toBeVisible();
    expect(
      screen.queryByRole("textbox", { name: "Offline username" }),
    ).not.toBeInTheDocument();
    const account = within(screen.getByRole("region", { name: "Account" }));
    expect(
      account.getByRole("heading", { name: "Sign in with Microsoft" }),
    ).toBeVisible();
    expect(account.getByRole("button", { name: "Sign in" })).toBeEnabled();
    expect(
      account.queryByRole("button", { name: "Offline username" }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "General" })).toBeVisible();
    expect(
      screen.getByText("Accounts, appearance and launcher behavior."),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Sign in" })).toBeInTheDocument();
  });

  it("starts Microsoft browser sign-in and shows the active account", async () => {
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
