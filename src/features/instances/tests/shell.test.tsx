import {
  act,
  fireEvent,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
  closeWindow,
  invoke,
  listeners,
  mockContextMenuCommands,
  renderAt,
  setupInstancesPageTests,
} from "@/features/instances/tests/support/render-instances";
import { playerHeadPathFixture } from "@/features/settings/fixtures/snapshot";

describe("Launcher shell", () => {
  setupInstancesPageTests();

  it("opens Updates settings from the footer version", async () => {
    mockContextMenuCommands();
    const { router } = await renderAt("/");
    fireEvent.click(
      within(
        screen.getByRole("contentinfo", { name: "Download status" }),
      ).getByRole("button", { name: "Early Release 0.3.1 · Rose" }),
    );
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("open_settings_window", {
        section: "updater",
      }),
    );
    expect(router.state.location.pathname).toBe("/");
  });

  it("opens settings from the Licht menu using the keyboard", async () => {
    mockContextMenuCommands();
    await renderAt("/");
    await screen.findByRole("heading", { name: "Classic" });
    const trigger = screen.getByRole("button", { name: "Licht Launcher" });
    trigger.focus();
    fireEvent.keyDown(trigger, { key: "Enter" });
    const menu = await screen.findByRole("menu", { name: "Licht menu" });
    const accounts = within(menu).getByRole("menuitem", {
      name: "Manage accounts",
    });
    const settings = within(menu).getByRole("menuitem", { name: "Settings" });
    await waitFor(() => expect(accounts).toHaveFocus());
    fireEvent.keyDown(accounts, { key: "ArrowDown" });
    await waitFor(() => expect(settings).toHaveFocus());
    fireEvent.keyDown(settings, { key: "Enter" });
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("open_settings_window", {
        section: "account",
      }),
    );
    expect(
      screen.queryByRole("menu", { name: "Licht menu" }),
    ).not.toBeInTheDocument();
    expect(closeWindow).not.toHaveBeenCalled();
  });

  it("requests explicit application exit from Exit", async () => {
    mockContextMenuCommands();
    await renderAt("/");
    await screen.findByRole("heading", { name: "Classic" });
    const trigger = screen.getByRole("button", { name: "Licht Launcher" });
    // Avoid hitting jsdom's zero-sized resize handle at the origin.
    fireEvent.pointerDown(trigger, {
      button: 0,
      ctrlKey: false,
      pointerType: "mouse",
      clientX: 50,
      clientY: 20,
    });
    fireEvent.pointerUp(trigger, {
      button: 0,
      pointerType: "mouse",
      clientX: 50,
      clientY: 20,
    });
    fireEvent.click(
      within(await screen.findByRole("menu", { name: "Licht menu" })).getByRole(
        "menuitem",
        { name: "Exit" },
      ),
    );
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("quit_app"));
    expect(closeWindow).not.toHaveBeenCalled();
  });

  it("opens feedback from the Licht menu and closes the menu", async () => {
    mockContextMenuCommands();
    await renderAt("/");
    const trigger = screen.getByRole("button", { name: "Licht Launcher" });
    trigger.focus();
    fireEvent.keyDown(trigger, { key: "Enter" });
    const menu = await screen.findByRole("menu", { name: "Licht menu" });
    fireEvent.click(
      within(menu).getByRole("menuitem", { name: "Send feedback" }),
    );
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("open_feedback"));
    await waitFor(() =>
      expect(trigger).toHaveAttribute("aria-expanded", "false"),
    );
  });

  it.each([
    {
      name: "OnlinePlayer",
      profile: {
        uuid: "0123456789abcdef0123456789abcdef",
        name: "OnlinePlayer",
        skinUrl: `https://textures.minecraft.net/texture/${"a".repeat(64)}`,
      },
    },
    { name: "Steve", profile: null },
  ])("shows $name and opens the account menu", async ({ name, profile }) => {
    mockContextMenuCommands((command) => {
      if (command === "active_microsoft_account")
        return Promise.resolve(profile);
      if (command === "list_microsoft_accounts")
        return Promise.resolve(profile ? [profile] : []);
    });
    await renderAt("/");
    const account = await screen.findByRole("button", {
      name: `Account: ${name}`,
    });
    expect(account).toHaveTextContent(name);
    await waitFor(() =>
      expect(account.querySelector("img")).toHaveAttribute(
        "src",
        `asset://localhost/${playerHeadPathFixture}`,
      ),
    );
    expect(invoke).toHaveBeenCalledWith("get_player_head", {
      skinUrl: profile?.skinUrl ?? null,
    });
    account.focus();
    fireEvent.keyDown(account, { key: "Enter" });
    const menu = await screen.findByRole("menu", { name: "Player accounts" });
    expect(
      within(menu).getByRole("menuitem", { name: "Add account" }),
    ).toBeVisible();
    if (profile) {
      expect(
        await within(menu).findByRole("menuitemradio", { name: profile.name }),
      ).toHaveAttribute("aria-checked", "true");
    }
    fireEvent.keyDown(menu, { key: "Escape" });
    await waitFor(() =>
      expect(
        screen.queryByRole("menu", { name: "Player accounts" }),
      ).not.toBeInTheDocument(),
    );
  });

  it("opens Account settings from Add account without starting login", async () => {
    mockContextMenuCommands();
    await renderAt("/");
    const account = await screen.findByRole("button", {
      name: "Account: Steve",
    });
    account.focus();
    fireEvent.keyDown(account, { key: "Enter" });
    const menu = await screen.findByRole("menu", { name: "Player accounts" });
    fireEvent.click(
      within(menu).getByRole("menuitem", { name: "Add account" }),
    );
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("open_settings_window", {
        section: "account",
      }),
    );
    expect(invoke).not.toHaveBeenCalledWith("begin_microsoft_login");
  });

  it("opens Download settings without leaving Downloads", async () => {
    mockContextMenuCommands();
    const { router } = await renderAt("/downloads");
    fireEvent.click(
      await screen.findByRole("button", { name: "Download settings" }),
    );
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("open_settings_window", {
        section: "downloads",
      }),
    );
    expect(router.state.location.pathname).toBe("/downloads");
  });

  it("shows an error when the settings window cannot open", async () => {
    mockContextMenuCommands((command) =>
      command === "open_settings_window"
        ? Promise.reject(new Error("Settings window failed"))
        : undefined,
    );
    await renderAt("/downloads");
    fireEvent.click(
      await screen.findByRole("button", { name: "Download settings" }),
    );
    expect(await screen.findByText("Settings window failed")).toBeVisible();
  });

  it("opens Downloads when Developer Tools previews storage loading", async () => {
    mockContextMenuCommands();
    const { router } = await renderAt("/");
    await waitFor(() =>
      expect(listeners.has("dev-storage-loading-preview")).toBe(true),
    );
    await act(async () => {
      listeners.get("dev-storage-loading-preview")?.({ payload: null });
      await router.load();
    });
    expect(
      await screen.findByRole("heading", { name: "Downloads" }),
    ).toBeVisible();
    expect(router.state.location.pathname).toBe("/downloads");
  });

  it("refetches the account when another window changes it", async () => {
    mockContextMenuCommands();
    await renderAt("/");
    expect(
      await screen.findByRole("button", { name: "Account: Steve" }),
    ).toBeVisible();
    await waitFor(() => expect(listeners.has("accounts-changed")).toBe(true));
    mockContextMenuCommands((command) =>
      command === "active_microsoft_account"
        ? Promise.resolve({
            uuid: "0123456789abcdef0123456789abcdef",
            name: "OnlinePlayer",
            skinUrl: null,
          })
        : undefined,
    );
    act(() => listeners.get("accounts-changed")?.({ payload: null }));
    expect(
      await screen.findByRole("button", { name: "Account: OnlinePlayer" }),
    ).toBeVisible();
  });
});
