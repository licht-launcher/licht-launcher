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
        selected
        onSelect={onSelect}
      />,
    );

    const button = screen.getByRole("button", { name: /From a modpack/ });
    expect(button).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(button);
    expect(onSelect).toHaveBeenCalledOnce();
    expect(button.querySelector(".text-primary")).toBeInTheDocument();
    expect(button.className).toContain("bg-primary/15");
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
    expect(well?.className).not.toContain("text-primary");
  });
});
