import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { Button } from "@/components/ui/button";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";

function renderMenu(onSelect = vi.fn()) {
  render(
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <Button>Open menu</Button>
      </ContextMenuTrigger>
      <ContextMenuContent aria-label="Test menu">
        <ContextMenuItem>First</ContextMenuItem>
        <ContextMenuItem disabled onSelect={onSelect}>
          Disabled
        </ContextMenuItem>
        <ContextMenuItem onSelect={onSelect}>Last</ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>,
  );
  return {
    trigger: screen.getByRole("button", { name: "Open menu" }),
    onSelect,
  };
}

describe("ContextMenu", () => {
  it("opens on right-click and prevents disabled items from running", async () => {
    const { trigger, onSelect } = renderMenu();
    fireEvent.contextMenu(trigger, { clientX: 50, clientY: 60 });
    expect(
      await screen.findByRole("menu", { name: "Test menu" }),
    ).toBeVisible();
    const disabled = screen.getByRole("menuitem", { name: "Disabled" });
    expect(disabled).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(disabled);
    expect(onSelect).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("menuitem", { name: "Last" }));
    expect(onSelect).toHaveBeenCalledTimes(1);
    await waitFor(() =>
      expect(screen.queryByRole("menu")).not.toBeInTheDocument(),
    );
  });

  it.each(["ContextMenu", "F10"])(
    "opens with %s and supports keyboard selection",
    async (key) => {
      const { trigger, onSelect } = renderMenu();
      trigger.focus();
      fireEvent.keyDown(trigger, { key, shiftKey: key === "F10" });
      await screen.findByRole("menu", { name: "Test menu" });
      const first = screen.getByRole("menuitem", { name: "First" });
      await waitFor(() => expect(first).toHaveFocus());
      fireEvent.keyDown(first, { key: "ArrowDown" });
      const last = screen.getByRole("menuitem", { name: "Last" });
      await waitFor(() => expect(last).toHaveFocus());
      fireEvent.keyDown(last, { key: "Enter" });
      expect(onSelect).toHaveBeenCalledTimes(1);
      await waitFor(() =>
        expect(screen.queryByRole("menu")).not.toBeInTheDocument(),
      );
    },
  );

  it("closes with Escape and restores the previous focus", async () => {
    const { trigger } = renderMenu();
    trigger.focus();
    fireEvent.keyDown(trigger, { key: "ContextMenu" });
    const menu = await screen.findByRole("menu");
    fireEvent.keyDown(menu, { key: "Escape" });
    await waitFor(() =>
      expect(screen.queryByRole("menu")).not.toBeInTheDocument(),
    );
    await waitFor(() => expect(trigger).toHaveFocus());
  });
});
