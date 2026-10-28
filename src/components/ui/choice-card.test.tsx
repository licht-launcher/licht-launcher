import { fireEvent, render, screen } from "@testing-library/react";
import { PackageIcon } from "lucide-react";
import { describe, expect, it, vi } from "vitest";

import { ChoiceCard } from "@/components/ui/choice-card";

describe("ChoiceCard", () => {
  it("runs the select handler and exposes pressed state", () => {
    const onSelect = vi.fn();

    const { rerender } = render(
      <ChoiceCard
        title="From a modpack"
        description="Browse packs"
        icon={<PackageIcon aria-hidden />}
        iconTone="warning"
        selected
        onSelect={onSelect}
      />,
    );

    const button = screen.getByRole("button", { name: /From a modpack/ });
    expect(button).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(button);
    expect(onSelect).toHaveBeenCalledOnce();
    rerender(
      <ChoiceCard
        title="From a modpack"
        description="Browse packs"
        icon={<PackageIcon aria-hidden />}
        iconTone="warning"
        selected={false}
        onSelect={onSelect}
      />,
    );
    expect(button).toHaveAttribute("aria-pressed", "false");
  });

  it("does not select when disabled", () => {
    const onSelect = vi.fn();
    render(
      <ChoiceCard
        title="Upload .mrpack"
        description="Import a local Modrinth pack file."
        iconTone="file"
        selected={false}
        disabled
        onSelect={onSelect}
      />,
    );

    const button = screen.getByRole("button", { name: /Upload \.mrpack/ });
    expect(button).toBeDisabled();
    fireEvent.click(button);
    expect(onSelect).not.toHaveBeenCalled();
  });
});
