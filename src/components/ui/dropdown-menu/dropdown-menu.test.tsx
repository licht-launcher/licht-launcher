import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

function renderMenu() {
  const onSelect = vi.fn();
  render(
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button>Licht</Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent aria-label="Test menu">
        <DropdownMenuItem>Settings</DropdownMenuItem>
        <DropdownMenuItem disabled onSelect={onSelect}>
          Unavailable
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem onSelect={onSelect}>Exit</DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>,
  );
  return { trigger: screen.getByRole("button", { name: "Licht" }), onSelect };
}

describe("DropdownMenu", () => {
  it("opens with the keyboard, skips disabled items and selects an action", async () => {
    const { trigger, onSelect } = renderMenu();
    trigger.focus();
    fireEvent.keyDown(trigger, { key: "Enter" });
    expect(
      await screen.findByRole("menu", { name: "Test menu" }),
    ).toBeVisible();
    const settings = screen.getByRole("menuitem", { name: "Settings" });
    await waitFor(() => expect(settings).toHaveFocus());
    const disabled = screen.getByRole("menuitem", { name: "Unavailable" });
    expect(disabled).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(disabled);
    expect(onSelect).not.toHaveBeenCalled();
    fireEvent.keyDown(settings, { key: "ArrowDown" });
    const exit = screen.getByRole("menuitem", { name: "Exit" });
    await waitFor(() => expect(exit).toHaveFocus());
    fireEvent.keyDown(exit, { key: "Enter" });
    expect(onSelect).toHaveBeenCalledOnce();
    await waitFor(() =>
      expect(screen.queryByRole("menu")).not.toBeInTheDocument(),
    );
    await waitFor(() => expect(trigger).toHaveFocus());
  });

  it("opens with a pointer and returns focus after Escape", async () => {
    const { trigger } = renderMenu();
    fireEvent.pointerDown(trigger, {
      button: 0,
      ctrlKey: false,
      pointerType: "mouse",
    });
    const menu = await screen.findByRole("menu", { name: "Test menu" });
    fireEvent.keyDown(menu, { key: "Escape" });
    await waitFor(() =>
      expect(screen.queryByRole("menu")).not.toBeInTheDocument(),
    );
    await waitFor(() => expect(trigger).toHaveFocus());
  });
});
