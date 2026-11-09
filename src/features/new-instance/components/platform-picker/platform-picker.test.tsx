import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { PlatformPicker } from "@/features/new-instance/components/platform-picker";

describe("PlatformPicker", () => {
  it("lists every platform and marks the chosen one", () => {
    render(<PlatformPicker value="fabric" onChange={() => {}} />);

    const names = screen
      .getAllByRole("button")
      .map((button) => button.textContent);
    expect(names).toEqual(["Vanilla", "Fabric", "NeoForge", "Forge", "Quilt"]);
    expect(screen.getByRole("button", { name: "Fabric" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "Forge" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("reports the platform that was picked", () => {
    const onChange = vi.fn();
    render(<PlatformPicker value="vanilla" onChange={onChange} />);

    fireEvent.click(screen.getByRole("button", { name: "Quilt" }));
    expect(onChange).toHaveBeenCalledWith("quilt");
  });

  it("flows the cards one after the other at the width of their content", () => {
    render(<PlatformPicker value="vanilla" onChange={() => {}} />);

    const group = screen.getByRole("group", { name: "Platform" });
    expect(group).toHaveClass("flex", "flex-wrap");
    expect(group).not.toHaveClass("grid");
    for (const card of screen.getAllByRole("button")) {
      expect(card).toHaveClass("w-auto");
    }
  });
});
