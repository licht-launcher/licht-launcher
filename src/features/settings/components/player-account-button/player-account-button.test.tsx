import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { toast } from "sonner";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { PlayerAccountButton } from "@/features/settings/components/player-account-button";
import {
  playerHeadPathFixture,
  snapshot,
} from "@/features/settings/fixtures/snapshot";
import type { AccountProfile } from "@/lib/generated/AccountProfile";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke,
  convertFileSrc: (path: string) => `asset://localhost/${path}`,
}));
vi.mock("sonner", () => ({ toast: { error: vi.fn() } }));

const first: AccountProfile = {
  uuid: "11111111111111111111111111111111",
  name: "FirstPlayer",
  skinUrl: `https://textures.minecraft.net/texture/${"a".repeat(64)}`,
};
const second: AccountProfile = {
  uuid: "22222222222222222222222222222222",
  name: "SecondPlayer",
  skinUrl: `https://textures.minecraft.net/texture/${"b".repeat(64)}`,
};
const secondHead = "C:\\Licht\\cache\\player-heads\\second.png";

function backend(
  profiles: AccountProfile[] = [first, second],
  active: AccountProfile | null = first,
) {
  const state = { profiles: [...profiles], active };
  invoke.mockImplementation(
    async (command: string, args?: Record<string, unknown>) => {
      if (command === "get_settings") return snapshot;
      if (command === "active_microsoft_account") return state.active;
      if (command === "list_microsoft_accounts") return state.profiles;
      if (command === "get_player_head")
        return args?.skinUrl === second.skinUrl
          ? secondHead
          : playerHeadPathFixture;
      if (command === "set_active_microsoft_account") {
        const found = state.profiles.find(
          (profile) => profile.uuid === args?.uuid,
        );
        if (!found) throw new Error("Unknown fixture account");
        state.active = found;
        return found;
      }
      if (command === "use_offline_account") {
        state.active = null;
        return undefined;
      }
      throw new Error(`Unexpected command ${command}`);
    },
  );
  return state;
}

function view(onAddAccount = vi.fn()) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <PlayerAccountButton onAddAccount={onAddAccount} />
    </QueryClientProvider>,
  );
}

async function open(name: string) {
  const button = await screen.findByRole("button", {
    name: `Account: ${name}`,
  });
  button.focus();
  fireEvent.keyDown(button, { key: "Enter" });
  return screen.findByRole("menu", { name: "Player accounts" });
}

describe("PlayerAccountButton", () => {
  beforeEach(() => {
    invoke.mockReset();
    vi.mocked(toast.error).mockReset();
  });

  it("lists saved accounts, marks the active one and switches its name and cached head", async () => {
    backend();
    view();
    const menu = await open("FirstPlayer");
    const current = await within(menu).findByRole("menuitemradio", {
      name: "FirstPlayer",
    });
    expect(current).toHaveAttribute("aria-checked", "true");
    const other = within(menu).getByRole("menuitemradio", {
      name: "SecondPlayer",
    });
    expect(other).toHaveAttribute("aria-checked", "false");
    expect(invoke).not.toHaveBeenCalledWith("get_player_head", {
      skinUrl: second.skinUrl,
    });
    fireEvent.keyDown(menu, { key: "Home" });
    await waitFor(() => expect(current).toHaveFocus());
    fireEvent.keyDown(current, { key: "ArrowDown" });
    await waitFor(() => expect(other).toHaveFocus());
    fireEvent.keyDown(other, { key: "Enter" });
    const selected = await screen.findByRole("button", {
      name: "Account: SecondPlayer",
    });
    await waitFor(() =>
      expect(selected.querySelector("img")).toHaveAttribute(
        "src",
        `asset://localhost/${secondHead}`,
      ),
    );
    expect(invoke).toHaveBeenCalledWith("set_active_microsoft_account", {
      uuid: second.uuid,
    });
    expect(invoke).toHaveBeenCalledWith("get_player_head", {
      skinUrl: second.skinUrl,
    });
    const reopened = await open("SecondPlayer");
    expect(
      await within(reopened).findByRole("menuitemradio", {
        name: "SecondPlayer",
      }),
    ).toHaveAttribute("aria-checked", "true");
    expect(
      within(reopened).getByRole("menuitemradio", { name: "FirstPlayer" }),
    ).toHaveAttribute("aria-checked", "false");
  });

  it("opens account settings from Add account instead of starting browser login", async () => {
    backend([], null);
    const onAdd = vi.fn();
    view(onAdd);
    const menu = await open("Steve");
    expect(
      await within(menu).findByText("No Microsoft accounts added"),
    ).toBeVisible();
    expect(
      within(menu).getByRole("menuitemradio", {
        name: "Offline account · Steve",
      }),
    ).toHaveAttribute("aria-checked", "true");
    fireEvent.click(
      within(menu).getByRole("menuitem", { name: "Add account" }),
    );
    expect(onAdd).toHaveBeenCalledOnce();
    expect(invoke).toHaveBeenCalledWith("get_player_head", { skinUrl: null });
    expect(invoke).not.toHaveBeenCalledWith("begin_microsoft_login");
    await waitFor(() =>
      expect(
        screen.queryByRole("menu", { name: "Player accounts" }),
      ).not.toBeInTheDocument(),
    );
  });

  it("selects offline mode without losing saved accounts and can select them again", async () => {
    const state = backend();
    view();
    const menu = await open("FirstPlayer");
    const offline = within(menu).getByRole("menuitemradio", {
      name: "Offline account · Steve",
    });
    expect(offline).toHaveAttribute("aria-checked", "false");
    fireEvent.click(offline);
    expect(
      await screen.findByRole("button", { name: "Account: Steve" }),
    ).toBeVisible();
    expect(invoke).toHaveBeenCalledWith("use_offline_account");
    expect(state.profiles).toEqual([first, second]);
    expect(
      invoke.mock.calls.some(
        ([command]) => command === "logout_microsoft_account",
      ),
    ).toBe(false);
    const reopened = await open("Steve");
    expect(
      within(reopened).getByRole("menuitemradio", {
        name: "Offline account · Steve",
      }),
    ).toHaveAttribute("aria-checked", "true");
    expect(
      await within(reopened).findByRole("menuitemradio", {
        name: "FirstPlayer",
      }),
    ).toHaveAttribute("aria-checked", "false");
    fireEvent.click(
      within(reopened).getByRole("menuitemradio", { name: "SecondPlayer" }),
    );
    expect(
      await screen.findByRole("button", { name: "Account: SecondPlayer" }),
    ).toBeVisible();
    expect(invoke).toHaveBeenCalledWith("set_active_microsoft_account", {
      uuid: second.uuid,
    });
  });

  it("keeps the current account when activation fails and reports the error", async () => {
    backend();
    const original = invoke.getMockImplementation();
    if (!original) throw new Error("Missing backend fixture");
    invoke.mockImplementation(
      (command: string, args?: Record<string, unknown>) =>
        command === "set_active_microsoft_account"
          ? Promise.reject(new Error("Account switch unavailable"))
          : original(command, args),
    );
    view();
    const menu = await open("FirstPlayer");
    fireEvent.click(
      await within(menu).findByRole("menuitemradio", { name: "SecondPlayer" }),
    );
    await waitFor(() =>
      expect(toast.error).toHaveBeenCalledWith("Account switch unavailable"),
    );
    expect(
      screen.getByRole("button", { name: "Account: FirstPlayer" }),
    ).toBeVisible();
  });

  it("reports a failed account list and retries without starting a login", async () => {
    backend();
    const original = invoke.getMockImplementation();
    if (!original) throw new Error("Missing backend fixture");
    let fail = true;
    invoke.mockImplementation(
      (command: string, args?: Record<string, unknown>) =>
        command === "list_microsoft_accounts" && fail
          ? Promise.reject(new Error("Account list unavailable"))
          : original(command, args),
    );
    view();
    const menu = await open("FirstPlayer");
    expect(await within(menu).findByRole("alert")).toHaveTextContent(
      "Could not load accounts.",
    );
    fail = false;
    fireEvent.click(within(menu).getByRole("menuitem", { name: "Retry" }));
    expect(
      await within(menu).findByRole("menuitemradio", { name: "FirstPlayer" }),
    ).toHaveAttribute("aria-checked", "true");
    expect(invoke).not.toHaveBeenCalledWith("begin_microsoft_login");
  });

  it("keeps the online account selected if switching to offline fails", async () => {
    backend();
    const original = invoke.getMockImplementation();
    if (!original) throw new Error("Missing backend fixture");
    invoke.mockImplementation(
      (command: string, args?: Record<string, unknown>) =>
        command === "use_offline_account"
          ? Promise.reject(new Error("Offline selection unavailable"))
          : original(command, args),
    );
    view();
    const menu = await open("FirstPlayer");
    fireEvent.click(
      within(menu).getByRole("menuitemradio", {
        name: "Offline account · Steve",
      }),
    );
    await waitFor(() =>
      expect(toast.error).toHaveBeenCalledWith("Offline selection unavailable"),
    );
    expect(
      screen.getByRole("button", { name: "Account: FirstPlayer" }),
    ).toBeVisible();
    const reopened = await open("FirstPlayer");
    expect(
      within(reopened).getByRole("menuitemradio", {
        name: "Offline account · Steve",
      }),
    ).toHaveAttribute("aria-checked", "false");
  });
});
