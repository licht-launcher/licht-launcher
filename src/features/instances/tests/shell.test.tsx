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
  mockContextMenuCommands,
  renderAt,
  setupInstancesPageTests,
} from "@/features/instances/tests/support/render-instances";
import { playerHeadPathFixture } from "@/features/settings/fixtures/snapshot";
import { requestOpenSettings } from "@/lib/shell-events";

describe("Launcher shell", () => {
  setupInstancesPageTests();

  it("opens Updater settings from the footer version", async () => {
    mockContextMenuCommands();
    await renderAt("/");
    fireEvent.click(
      within(
        screen.getByRole("contentinfo", { name: "Download status" }),
      ).getByRole("button", { name: "Early Release 0.3.0 · Rose" }),
    );
    const settings = await screen.findByRole("dialog", { name: "Settings" });
    expect(
      within(settings).getByRole("button", {
        name: "Updater",
        current: "page",
      }),
    ).toBeVisible();
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
    expect(
      await screen.findByRole("dialog", { name: "Settings" }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Appearance", current: "page" }),
    ).toBeVisible();
    expect(
      screen.queryByRole("menu", { name: "Licht menu" }),
    ).not.toBeInTheDocument();
    expect(closeWindow).not.toHaveBeenCalled();
  });

  it("requests window closure from Exit", async () => {
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
    await waitFor(() => expect(closeWindow).toHaveBeenCalledOnce());
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
    const settings = await screen.findByRole("dialog", { name: "Settings" });
    expect(
      within(settings).getByRole("button", {
        name: "Account",
        current: "page",
      }),
    ).toBeVisible();
    expect(invoke).not.toHaveBeenCalledWith("begin_microsoft_login");
  });

  it("opens Download settings without leaving Downloads", async () => {
    mockContextMenuCommands();
    const { router } = await renderAt("/downloads");
    fireEvent.click(
      await screen.findByRole("button", { name: "Download settings" }),
    );
    const settings = await screen.findByRole("dialog", { name: "Settings" });
    expect(
      within(settings).getByRole("button", {
        name: "Downloads",
        current: "page",
      }),
    ).toBeVisible();
    expect(router.state.location.pathname).toBe("/downloads");
  });

  it("opens the developer loading preview in Downloads and closes Settings", async () => {
    mockContextMenuCommands();
    const { router } = await renderAt("/");
    act(() => requestOpenSettings("developers"));
    const settings = await screen.findByRole("dialog", { name: "Settings" });
    const preview = await within(settings).findByRole("button", {
      name: "Test Launcher Data loading",
    });
    await act(async () => {
      fireEvent.click(preview);
      await router.load();
    });
    expect(
      await screen.findByRole("heading", { name: "Downloads" }),
    ).toBeVisible();
    expect(
      screen.queryByRole("dialog", { name: "Settings" }),
    ).not.toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/downloads");
    const data = within(screen.getByRole("region", { name: "Launcher Data" }));
    expect(data.getAllByRole("status")).toHaveLength(6);
    expect(data.getByText("Instances")).toBeVisible();
    expect(invoke).not.toHaveBeenCalledWith("storage_summary");
  });
});
