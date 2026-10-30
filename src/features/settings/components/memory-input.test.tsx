import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { MemoryInput } from "@/features/settings/components/memory-input";

describe("MemoryInput", () => {
  it("accepts exact MB values independently of the slider's drag step", () => {
    const onChange = vi.fn();
    render(
      <MemoryInput
        valueMb={2048}
        minimumMb={512}
        maximumMb={32701}
        onChange={onChange}
      />,
    );
    const input = screen.getByRole("spinbutton", {
      name: "Memory Allocation (MB)",
    });
    fireEvent.change(input, { target: { value: "5001" } });
    expect(input).toHaveValue(5001);
    expect(onChange).toHaveBeenLastCalledWith(5001);
  });

  it.each([
    ["100", 512],
    ["40000", 32701],
  ])(
    "clamps %s to %s MB on blur without saving intermediate invalid values",
    (text, expected) => {
      const onChange = vi.fn();
      render(
        <MemoryInput
          valueMb={2048}
          minimumMb={512}
          maximumMb={32701}
          onChange={onChange}
        />,
      );
      const input = screen.getByRole("spinbutton", {
        name: "Memory Allocation (MB)",
      });
      fireEvent.change(input, { target: { value: text } });
      expect(onChange).not.toHaveBeenCalled();
      fireEvent.blur(input);
      expect(input).toHaveValue(expected);
      expect(onChange).toHaveBeenCalledExactlyOnceWith(expected);
    },
  );

  it("restores an empty edit and follows values changed by the slider", () => {
    const onChange = vi.fn();
    const view = render(
      <MemoryInput
        valueMb={2048}
        minimumMb={512}
        maximumMb={32701}
        onChange={onChange}
      />,
    );
    const input = screen.getByRole("spinbutton", {
      name: "Memory Allocation (MB)",
    });
    fireEvent.change(input, { target: { value: "" } });
    fireEvent.blur(input);
    expect(input).toHaveValue(2048);
    expect(onChange).not.toHaveBeenCalled();
    view.rerender(
      <MemoryInput
        valueMb={20001}
        minimumMb={512}
        maximumMb={32701}
        onChange={onChange}
      />,
    );
    expect(input).toHaveValue(20001);
  });
});
