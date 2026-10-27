import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  invoke,
  mockSettingsCommands,
  renderSettings,
} from "@/features/settings/fixtures/render-settings";
import { SettingsDialog } from "@/features/settings/settings-dialog";
import { requestOpenSettings } from "@/lib/shell-events";

vi.mock("@tauri-apps/api/core", async () => {
  const { invoke: mocked } = await import(
    "@/features/settings/fixtures/invoke"
  );
  return { invoke: mocked };
});

describe("SettingsDialog", () => {
  beforeEach(() => {
    mockSettingsCommands();
  });

  it("does not show a Save button", async () => {
    await renderSettings();
    expect(
      await screen.findByLabelText("Offline username"),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Save" }),
    ).not.toBeInTheDocument();
  });

  it("preserves edited fields and active input focus across section transitions", async () => {
    mockSettingsCommands((command, args) => {
      if (
        command === "save_settings" &&
        args !== null &&
        typeof args === "object" &&
        "settings" in args
      ) {
        return Promise.resolve(args.settings);
      }
      return undefined;
    });
    await renderSettings();
    fireEvent.change(await screen.findByLabelText("Offline username"), {
      target: { value: "Builder" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Downloads" }));
    const limit = await screen.findByLabelText(
      "Global download speed limit (MiB/s)",
    );
    limit.focus();
    fireEvent.change(limit, { target: { value: "8" } });
    expect(screen.getByLabelText("Global download speed limit (MiB/s)")).toBe(
      limit,
    );
    expect(limit).toHaveFocus();
    expect(limit).toHaveValue(8);

    fireEvent.click(screen.getByRole("button", { name: "Account" }));
    expect(await screen.findByLabelText("Offline username")).toHaveValue(
      "Builder",
    );
    fireEvent.click(screen.getByRole("button", { name: "Downloads" }));
    expect(
      await screen.findByLabelText("Global download speed limit (MiB/s)"),
    ).toHaveValue(8);
  });

  it("does not save just by opening", async () => {
    await renderSettings();
    await screen.findByLabelText("Offline username");
    await new Promise((resolve) => setTimeout(resolve, 500));
    expect(invoke).not.toHaveBeenCalledWith("save_settings", expect.anything());
  });

  it("keeps the application titlebar interactive while open", async () => {
    await renderSettings();
    await screen.findByLabelText("Offline username");

    expect(document.body).not.toHaveStyle({ pointerEvents: "none" });
  });

  it("keeps the modal backdrop visible while open", async () => {
    await renderSettings();
    await screen.findByLabelText("Offline username");

    expect(document.querySelector(".bg-black\\/60")).toBeInTheDocument();
  });

  it("does not close when the titlebar is clicked", async () => {
    await renderSettings();
    await screen.findByLabelText("Offline username");
    render(
      <header data-title-bar>
        <button type="button">Titlebar action</button>
      </header>,
    );

    fireEvent.pointerDown(
      screen.getByRole("button", { name: "Titlebar action" }),
    );
    fireEvent.pointerUp(
      screen.getByRole("button", { name: "Titlebar action" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Titlebar action" }));

    expect(screen.getByLabelText("Offline username")).toBeInTheDocument();
  });

  it("navigates to account when requested while already open", async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    render(
      <QueryClientProvider client={queryClient}>
        <SettingsDialog open onOpenChange={() => {}} initialSection="updater" />
      </QueryClientProvider>,
    );

    expect(
      await screen.findByRole("heading", { name: "Updater" }),
    ).toBeVisible();

    requestOpenSettings("account");

    expect(
      await screen.findByRole("heading", { name: "Account" }),
    ).toBeVisible();
  });
});
