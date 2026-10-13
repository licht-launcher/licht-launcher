import { fireEvent, render, screen } from "@testing-library/react";
import { PackageIcon } from "lucide-react";
import { describe, expect, it, vi } from "vitest";

import { ChoiceCard } from "@/components/ui/choice-card";

describe("ChoiceCard", () => {
  it("runs the select handler and exposes pressed state", () => {
    const onSelect = vi.fn();

    render(
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
    expect(button.className).toContain("bg-muted");
    expect(button.className).toContain("ring-2");
    expect(button.className).toContain("ring-primary");
    expect(button.className).not.toContain("border-primary");
    expect(button.querySelector(".text-warning")).toBeInTheDocument();
  });

  it("keeps loader icon wells neutral", () => {
    render(
      <ChoiceCard
        title="Fabric"
        icon={<img alt="" src="fabric.png" />}
        iconNeutral
        selected={false}
        onSelect={() => {}}
      />,
    );

    const button = screen.getByRole("button", { name: "Fabric" });
    const well = button.querySelector(".bg-black\\/20");
    expect(well).toBeInTheDocument();
    expect(well?.className).not.toContain("grayscale");
    expect(well?.className).not.toContain("text-instance");
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
    expect(button.className).toContain("disabled:opacity-50");
    fireEvent.click(button);
    expect(onSelect).not.toHaveBeenCalled();
  });
});
