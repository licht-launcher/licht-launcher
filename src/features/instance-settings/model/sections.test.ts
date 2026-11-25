import { describe, expect, it } from "vitest";

import { shownSection, visibleSections } from "./sections";

describe("instance settings sections", () => {
  it("offers the Modpack section only to linked instances", () => {
    expect(visibleSections(true).map((section) => section.id)).toContain(
      "modpack",
    );
    expect(visibleSections(false).map((section) => section.id)).not.toContain(
      "modpack",
    );
  });

  it("falls back to General when the Modpack section goes away", () => {
    expect(shownSection("modpack", false)).toBe("general");
    expect(shownSection("modpack", true)).toBe("modpack");
    expect(shownSection("base", false)).toBe("base");
  });
});
