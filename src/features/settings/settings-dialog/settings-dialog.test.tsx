import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
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

  it("groups accounts and appearance under General in order", async () => {
    await renderSettings();
    await screen.findByRole("button", { name: "Dark" });

    expect(screen.getByRole("heading", { name: "General" })).toBeVisible();
    expect(
      screen
        .getAllByRole("heading", { level: 3 })
        .map((heading) => heading.textContent),
    ).toEqual(["Account", "Appearance", "Behavior"]);
    expect(screen.getByRole("button", { name: "Dark" })).toHaveTextContent(
      "Dark",
    );
  });

  it("does not show a Save button", async () => {
    await renderSettings();
    expect(
      await screen.findByRole("button", { name: "Dark" }),
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
    fireEvent.click(
      await screen.findByRole("button", { name: "Developer Tools" }),
    );
    fireEvent.change(await screen.findByLabelText("Offline username"), {
      target: { value: "Builder" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Downloads" }));
    fireEvent.click(
      screen.getByRole("switch", { name: "Limit download speed" }),
    );
    const limit = await screen.findByLabelText("Speed limit (kB/s)");
    limit.focus();
    fireEvent.change(limit, { target: { value: "8" } });
    expect(screen.getByLabelText("Speed limit (kB/s)")).toBe(limit);
    expect(limit).toHaveFocus();
    expect(limit).toHaveValue(8);

    fireEvent.click(screen.getByRole("button", { name: "Developer Tools" }));
    expect(await screen.findByLabelText("Offline username")).toHaveValue(
      "Builder",
    );
    fireEvent.click(screen.getByRole("button", { name: "Downloads" }));
    expect(await screen.findByLabelText("Speed limit (kB/s)")).toHaveValue(8);
  });

  it("shows the offline username directly in Developer Tools and saves edits", async () => {
    mockSettingsCommands((command, args) => {
      if (
        command === "save_settings" &&
        typeof args === "object" &&
        args !== null &&
        "settings" in args
      ) {
        return Promise.resolve(args.settings);
      }
      return undefined;
    });
    await renderSettings();
    await screen.findByRole("button", { name: "Dark" });
    expect(
      screen.queryByRole("button", { name: "Offline username" }),
    ).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Developer Tools" }));
    expect(
      await screen.findByRole("heading", { name: "Developer Tools" }),
    ).toBeVisible();
    fireEvent.change(
      screen.getByRole("textbox", { name: "Offline username" }),
      { target: { value: "Builder" } },
    );
    expect(
      screen.getByRole("textbox", { name: "Offline username" }),
    ).toHaveValue("Builder");
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({ username: "Builder" }),
      });
    });
  });

  it("does not save just by opening", async () => {
    await renderSettings();
    await screen.findByRole("button", { name: "Dark" });
    await new Promise((resolve) => setTimeout(resolve, 500));
    expect(invoke).not.toHaveBeenCalledWith("save_settings", expect.anything());
  });

  it("makes background application controls inaccessible while Settings is open", async () => {
    render(<button type="button">Background application action</button>);
    await renderSettings();
    await screen.findByRole("button", { name: "Dark" });

    expect(
      screen.queryByRole("button", { name: "Background application action" }),
    ).not.toBeInTheDocument();
  });

  it("keeps the modal backdrop visible while open", async () => {
    await renderSettings();
    await screen.findByRole("button", { name: "Dark" });

    expect(document.querySelector(".bg-black\\/60")).toBeInTheDocument();
  });

  it("keeps window controls usable without dismissing Settings", async () => {
    const minimize = vi.fn();
    function Example() {
      const [open, setOpen] = useState(true);
      return (
        <>
          <div data-window-controls>
            <button type="button" onClick={minimize}>
              Minimize window
            </button>
          </div>
          <SettingsDialog open={open} onOpenChange={setOpen} />
        </>
      );
    }
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    render(
      <QueryClientProvider client={client}>
        <Example />
      </QueryClientProvider>,
    );
    await screen.findByRole("button", { name: "Dark" });
    const control = screen.getByText("Minimize window");
    fireEvent.pointerDown(control);
    fireEvent.pointerUp(control);
    fireEvent.click(control);
    expect(minimize).toHaveBeenCalledOnce();
    expect(screen.getByRole("dialog", { name: "Settings" })).toBeVisible();
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
      await screen.findByRole("heading", { name: "Updates" }),
    ).toBeVisible();

    requestOpenSettings("account");

    expect(
      await screen.findByRole("heading", { name: "General" }),
    ).toBeVisible();
  });
});
