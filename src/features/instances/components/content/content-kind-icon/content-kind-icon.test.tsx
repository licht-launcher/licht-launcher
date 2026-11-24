import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { ContentKindIcon } from "./content-kind-icon";

describe("ContentKindIcon", () => {
  it.each([
    ["mods", "text-accent-violet"],
    ["resourcePacks", "text-accent-orange"],
    ["shaderPacks", "text-accent-teal"],
  ] as const)("tints %s with its own accent", (kind, tint) => {
    const { container } = render(<ContentKindIcon kind={kind} />);

    const well = container.querySelector(`[data-kind=${kind}]`);
    expect(well).toHaveAttribute("aria-hidden", "true");
    expect(well?.querySelector("svg")).toHaveClass(tint, "size-4");
  });

  it("has a smaller size for inline counts", () => {
    const { container } = render(<ContentKindIcon kind="mods" size="sm" />);

    expect(container.querySelector("[data-kind=mods]")).toHaveClass("size-6");
  });
});
