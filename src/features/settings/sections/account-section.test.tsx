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

  it("starts Microsoft device-code sign-in", async () => {
    mockSettingsCommands((command) => {
      if (command === "list_microsoft_accounts") {
        return Promise.resolve([]);
      }
      if (command === "active_microsoft_account") {
        return Promise.resolve(null);
      }
      if (command === "begin_microsoft_login") {
        return Promise.resolve({
          userCode: "ABCD-1234",
          verificationUri: "https://microsoft.com/link",
          deviceCode: "device",
          intervalSeconds: 5,
          expiresInSeconds: 900,
        });
      }
      if (command === "poll_microsoft_login") {
        return Promise.resolve(null);
      }
      return undefined;
    });

    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Sign in" }));

    await waitFor(() => {
      expect(screen.getByText("ABCD-1234")).toBeInTheDocument();
    });
    expect(invoke).toHaveBeenCalledWith("begin_microsoft_login");
  });
});
