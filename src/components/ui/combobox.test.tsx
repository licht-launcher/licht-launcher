import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Combobox, type ComboboxOption } from "@/components/ui/combobox";

const options: ComboboxOption[] = [
  { value: "1.21", label: "1.21" },
  { value: "1.20.1", label: "1.20.1" },
  { value: "1.19.4", label: "1.19.4" },
];

beforeEach(() => {
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(
    function (this: HTMLElement) {
      return this.dataset.slot === "combobox-viewport" ? 256 : 0;
    },
  );
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(320);
});

afterEach(() => {
  vi.restoreAllMocks();
});

function Harness({
  list = options,
  initial = "1.20.1",
  onValueChange = () => {},
}: {
  list?: ComboboxOption[];
  initial?: string;
  onValueChange?: (value: string) => void;
}) {
  const [value, setValue] = useState(initial);
  return (
    <Combobox
      aria-label="Game version"
      options={list}
      value={value}
      emptyText="No versions match."
      onValueChange={(next) => {
        setValue(next);
        onValueChange(next);
      }}
    />
  );
}

describe("Combobox", () => {
  it("shows the selected option and opens the list on focus", () => {
    render(<Harness />);

    const input = screen.getByRole("combobox", { name: "Game version" });
    expect(input).toHaveValue("1.20.1");
    expect(input).toHaveAttribute("aria-expanded", "false");
    fireEvent.focus(input);
    expect(input).toHaveAttribute("aria-expanded", "true");
    expect(screen.getAllByRole("option")).toHaveLength(3);
    expect(screen.getByRole("option", { name: "1.20.1" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
  });

  it("selects an option by click and closes", () => {
    const onValueChange = vi.fn();
    render(<Harness onValueChange={onValueChange} />);

    const input = screen.getByRole("combobox", { name: "Game version" });
    fireEvent.focus(input);
    fireEvent.click(screen.getByRole("option", { name: "1.19.4" }));
    expect(onValueChange).toHaveBeenCalledWith("1.19.4");
    expect(input).toHaveValue("1.19.4");
    expect(input).toHaveAttribute("aria-expanded", "false");
  });

  it("filters while typing and explains an empty result", () => {
    render(<Harness />);

    const input = screen.getByRole("combobox", { name: "Game version" });
    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: "1.2" } });
    expect(screen.getAllByRole("option").map((o) => o.textContent)).toEqual([
      "1.21",
      "1.20.1",
    ]);
    fireEvent.change(input, { target: { value: "nope" } });
    expect(screen.queryAllByRole("option")).toHaveLength(0);
    expect(screen.getByText("No versions match.")).toBeVisible();
  });

  it("moves with the arrow keys and picks with Enter", () => {
    const onValueChange = vi.fn();
    render(<Harness onValueChange={onValueChange} />);

    const input = screen.getByRole("combobox", { name: "Game version" });
    fireEvent.focus(input);
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onValueChange).toHaveBeenCalledWith("1.19.4");
  });

  it("closes on Escape and restores the selected label", () => {
    const onValueChange = vi.fn();
    render(<Harness onValueChange={onValueChange} />);

    const input = screen.getByRole("combobox", { name: "Game version" });
    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: "1.1" } });
    fireEvent.keyDown(input, { key: "Escape" });
    expect(input).toHaveAttribute("aria-expanded", "false");
    expect(input).toHaveValue("1.20.1");
    expect(onValueChange).not.toHaveBeenCalled();
  });

  it("restores the selected label when it loses focus", () => {
    render(<Harness />);

    const input = screen.getByRole("combobox", { name: "Game version" });
    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: "xyz" } });
    fireEvent.blur(input);
    expect(input).toHaveValue("1.20.1");
    expect(input).toHaveAttribute("aria-expanded", "false");
  });

  it("only renders the rows that fit instead of every option", () => {
    const many: ComboboxOption[] = Array.from({ length: 800 }, (_, index) => ({
      value: `1.${index}`,
      label: `1.${index}`,
    }));
    render(<Harness list={many} initial="1.0" />);

    fireEvent.focus(screen.getByRole("combobox", { name: "Game version" }));
    const rows = screen.getAllByRole("option");
    expect(rows.length).toBeGreaterThan(0);
    expect(rows.length).toBeLessThan(60);
  });

  it("does not open when disabled", () => {
    render(
      <Combobox
        aria-label="Game version"
        options={options}
        value=""
        disabled
        onValueChange={() => {}}
      />,
    );

    const input = screen.getByRole("combobox", { name: "Game version" });
    expect(input).toBeDisabled();
    fireEvent.focus(input);
    expect(screen.queryAllByRole("option")).toHaveLength(0);
  });
});
