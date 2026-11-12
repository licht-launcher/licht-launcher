import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { ModpackGallery } from "./modpack-gallery";

describe("ModpackGallery", () => {
  it("captions images that have a title and keeps the rest decorative", () => {
    render(
      <ModpackGallery
        images={[
          { url: "https://cdn.example.com/title.webp", title: "Title screen" },
          { url: "https://cdn.example.com/plain.webp", title: null },
        ]}
      />,
    );

    const items = within(
      screen.getByRole("list", { name: "Gallery" }),
    ).getAllByRole("listitem");
    expect(items).toHaveLength(2);
    expect(
      within(items[0] as HTMLElement).getByRole("img", {
        name: "Title screen",
      }),
    ).toHaveAttribute("src", "https://cdn.example.com/title.webp");
    expect(
      within(items[0] as HTMLElement).getByText("Title screen", {
        selector: "figcaption",
      }),
    ).toBeInTheDocument();
    expect((items[1] as HTMLElement).querySelector("img")).toHaveAttribute(
      "alt",
      "",
    );
    expect(
      (items[1] as HTMLElement).querySelector("figcaption"),
    ).not.toBeInTheDocument();
  });
});
