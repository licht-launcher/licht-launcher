import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { SourcePicker } from "@/features/new-instance/components/source-picker";

function renderPicker(props: Partial<Parameters<typeof SourcePicker>[0]> = {}) {
  const onScratch = vi.fn();
  const onUpload = vi.fn();
  render(
    <SourcePicker
      busy={false}
      onScratch={onScratch}
      onUpload={onUpload}
      {...props}
    />,
  );
  return { onScratch, onUpload };
}

describe("SourcePicker", () => {
  it("lists the sources one below the other in order", () => {
    renderPicker();

    const list = screen.getByRole("list", { name: "Create from" });
    const rows = within(list).getAllByRole("listitem");
    expect(rows).toHaveLength(3);
    expect(rows[0]).toHaveTextContent("From scratch");
    expect(rows[1]).toHaveTextContent("From a modpack");
    expect(rows[2]).toHaveTextContent("Upload a pack file");
  });

  it("highlights the key words and file extensions", () => {
    renderPicker();

    for (const word of ["platform", "name", "game version", "Modrinth"]) {
      expect(screen.getByText(word, { selector: "strong" })).toBeVisible();
    }
    expect(screen.getByText(".mrpack", { selector: "code" })).toBeVisible();
    expect(screen.getByText(".zip", { selector: "code" })).toBeVisible();
    const word = screen.getByText("platform", { selector: "strong" });
    expect(word).toHaveClass("font-medium");
    expect(word).not.toHaveClass("font-semibold");
  });

  it("sizes the title above its description with a small gap between them", () => {
    renderPicker();

    const title = screen.getByText("From scratch");
    expect(title).toHaveClass("text-[15px]");
    expect(title.parentElement).toHaveClass("gap-0.5");
    expect(title.nextElementSibling).toHaveClass("text-xs");
  });

  it("keeps the modpack source disabled until Discover exists", () => {
    renderPicker();

    expect(
      screen.getByRole("button", { name: /From a modpack/ }),
    ).toBeDisabled();
  });

  it("starts the matching flow when a row is chosen", () => {
    const { onScratch, onUpload } = renderPicker();

    fireEvent.click(screen.getByRole("button", { name: /From scratch/ }));
    expect(onScratch).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("button", { name: /Upload a pack file/ }));
    expect(onUpload).toHaveBeenCalledOnce();
  });

  it("disables the available sources while something is being created", () => {
    renderPicker({ busy: true });

    expect(screen.getByRole("button", { name: /From scratch/ })).toBeDisabled();
    expect(
      screen.getByRole("button", { name: /Upload a pack file/ }),
    ).toBeDisabled();
  });
});
