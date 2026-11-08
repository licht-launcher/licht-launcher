import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

describe("Select", () => {
  it.each(["item-aligned", "popper"] as const)(
    "selects with the keyboard and restores focus in %s mode",
    async (position) => {
      render(
        <Select defaultValue="release">
          <SelectTrigger aria-label="Release channel">
            <SelectValue />
          </SelectTrigger>
          <SelectContent position={position}>
            <SelectItem value="release">Release</SelectItem>
            <SelectItem value="beta">Beta</SelectItem>
          </SelectContent>
        </Select>,
      );
      const trigger = screen.getByRole("combobox", { name: "Release channel" });
      trigger.focus();
      fireEvent.keyDown(trigger, { key: "ArrowDown" });
      const release = await screen.findByRole("option", { name: "Release" });
      await waitFor(() => expect(release).toHaveFocus());
      fireEvent.keyDown(release, { key: "ArrowDown" });
      const beta = screen.getByRole("option", { name: "Beta" });
      await waitFor(() => expect(beta).toHaveFocus());
      fireEvent.keyDown(beta, { key: "Enter" });
      await waitFor(() => expect(screen.queryByRole("listbox")).toBeNull());
      expect(trigger).toHaveTextContent("Beta");
      await waitFor(() => expect(trigger).toHaveFocus());
    },
  );

  it.each(["item-aligned", "popper"] as const)(
    "closes with Escape and restores focus without changing the value in %s mode",
    async (position) => {
      render(
        <Select defaultValue="release">
          <SelectTrigger aria-label="Release channel">
            <SelectValue />
          </SelectTrigger>
          <SelectContent position={position}>
            <SelectItem value="release">Release</SelectItem>
            <SelectItem value="beta">Beta</SelectItem>
          </SelectContent>
        </Select>,
      );
      const trigger = screen.getByRole("combobox", { name: "Release channel" });
      fireEvent.click(trigger);
      const options = await screen.findByRole("listbox");
      fireEvent.keyDown(options, { key: "Escape" });
      await waitFor(() => expect(screen.queryByRole("listbox")).toBeNull());
      expect(trigger).toHaveTextContent("Release");
      await waitFor(() => expect(trigger).toHaveFocus());
    },
  );

  it("chooses an option", async () => {
    render(
      <Select>
        <SelectTrigger aria-label="Canal">
          <SelectValue placeholder="Escolha" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="release">Release</SelectItem>
        </SelectContent>
      </Select>,
    );

    fireEvent.click(screen.getByRole("combobox", { name: "Canal" }));
    fireEvent.click(await screen.findByRole("option", { name: "Release" }));

    expect(screen.getByRole("combobox", { name: "Canal" })).toHaveTextContent(
      "Release",
    );
  });
});
