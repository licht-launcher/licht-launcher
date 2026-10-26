import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SegmentedBar } from "@/components/ui/segmented-bar";

describe("SegmentedBar", () => {
  it("shows proportional colored segments and a readable category breakdown", () => {
    render(
      <SegmentedBar
        label="Storage"
        segments={[
          {
            label: "Instances",
            value: 2,
            valueLabel: "2 GB",
            color: "primary",
          },
          {
            label: "Game files",
            value: 3,
            valueLabel: "3 GB",
            color: "warning",
          },
          { label: "Java", value: 5, valueLabel: "5 GB", color: "chart1" },
        ]}
      />,
    );
    expect(
      screen.getByRole("img", {
        name: "Storage. Instances: 2 GB; Game files: 3 GB; Java: 5 GB",
      }),
    ).toBeVisible();
    expect(screen.getByLabelText("Instances segment")).toHaveAttribute(
      "width",
      "200",
    );
    expect(screen.getByLabelText("Instances segment")).toHaveClass(
      "fill-primary",
    );
    expect(screen.getByLabelText("Game files segment")).toHaveAttribute(
      "width",
      "300",
    );
    expect(screen.getByLabelText("Game files segment")).toHaveClass(
      "fill-warning",
    );
    expect(screen.getByLabelText("Java segment")).toHaveAttribute(
      "width",
      "500",
    );
    expect(
      screen.getByRole("list", { name: "Storage breakdown" }),
    ).toHaveTextContent("Java5 GB");
  });

  it("keeps zero-size categories in the legend without drawing occupied space", () => {
    render(
      <SegmentedBar
        label="Storage"
        segments={[
          { label: "Instances", value: 0, valueLabel: "0 B", color: "primary" },
          { label: "Cache", value: 0, valueLabel: "0 B", color: "chart2" },
        ]}
      />,
    );
    expect(
      screen.getByRole("img", { name: "Storage. Instances: 0 B; Cache: 0 B" }),
    ).toBeVisible();
    expect(
      screen.queryByLabelText("Instances segment"),
    ).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Cache segment")).not.toBeInTheDocument();
    expect(screen.getAllByRole("listitem")).toHaveLength(2);
  });

  it("keeps the outer ends fully rounded before and during hover", () => {
    render(
      <SegmentedBar
        label="Storage"
        segments={[
          {
            label: "Instances",
            value: 2,
            valueLabel: "2 GB",
            color: "primary",
          },
          { label: "Java", value: 5, valueLabel: "5 GB", color: "violet" },
        ]}
      />,
    );
    const last = screen.getByLabelText("Java segment");
    const bar = screen.getByLabelText("Storage bar");
    const highlight = screen.getByLabelText("Storage highlight");
    expect(bar).toHaveClass("rounded-full");
    fireEvent.mouseEnter(last);
    expect(bar).toHaveClass("rounded-full");
    expect(highlight).toHaveClass("rounded-full");
    expect(highlight).toHaveAttribute("height", "16");
  });

  it("keeps all segments within a single constrained viewport", () => {
    render(
      <SegmentedBar
        label="Storage"
        segments={[
          {
            label: "Instances",
            value: 2,
            valueLabel: "2 GB",
            color: "primary",
          },
          { label: "Java", value: 5, valueLabel: "5 GB", color: "violet" },
        ]}
      />,
    );
    const bar = screen.getByLabelText("Storage bar");
    const first = screen.getByLabelText("Instances segment");
    const last = screen.getByLabelText("Java segment");
    expect(bar).toHaveClass("w-full", "min-w-0", "max-w-full");
    expect(first.closest("svg")).toBe(bar);
    expect(last.closest("svg")).toBe(bar);
    expect(
      Number(last.getAttribute("x")) + Number(last.getAttribute("width")),
    ).toBe(1000);
  });

  it("fills the bar with a fully rounded single category when the others are empty", () => {
    render(
      <SegmentedBar
        label="Storage"
        segments={[
          { label: "Instances", value: 0, valueLabel: "0 B", color: "primary" },
          { label: "Java", value: 5, valueLabel: "5 GB", color: "violet" },
          { label: "Cache", value: 0, valueLabel: "0 B", color: "chart2" },
        ]}
      />,
    );
    const java = screen.getByLabelText("Java segment");
    expect(java).toHaveAttribute("width", "1000");
    expect(screen.getByLabelText("Storage bar")).toHaveClass("rounded-full");
    fireEvent.mouseEnter(java);
    expect(java).toHaveAttribute("width", "1000");
    expect(screen.getByLabelText("Storage highlight")).toHaveAttribute(
      "height",
      "16",
    );
  });

  it("clips the complete bar to rounded ends even when the final category is tiny", () => {
    render(
      <SegmentedBar
        label="Storage"
        segments={[
          {
            label: "Instances",
            value: 99999,
            valueLabel: "99.9 KB",
            color: "primary",
          },
          { label: "Other", value: 1, valueLabel: "1 B", color: "chart3" },
        ]}
      />,
    );
    const other = screen.getByLabelText("Other segment");
    const bar = screen.getByLabelText("Storage bar");
    expect(bar).toHaveClass("rounded-full", "overflow-hidden");
    expect(other.closest("svg")).toBe(bar);
    expect(
      Number(other.getAttribute("x")) + Number(other.getAttribute("width")),
    ).toBe(1000);
    fireEvent.mouseEnter(other);
    expect(screen.getByLabelText("Storage highlight")).toHaveClass(
      "rounded-full",
      "overflow-hidden",
    );
  });

  it("highlights the segment associated with a hovered value and restores the bar on exit", () => {
    render(
      <SegmentedBar
        label="Storage"
        segments={[
          {
            label: "Instances",
            value: 2,
            valueLabel: "2 GB",
            color: "primary",
          },
          { label: "Java", value: 5, valueLabel: "5 GB", color: "violet" },
        ]}
      />,
    );
    fireEvent.mouseEnter(screen.getByText("5 GB"));
    expect(screen.getByLabelText("Java segment")).toHaveClass(
      "opacity-100",
      "fill-accent-violet",
    );
    expect(screen.getByLabelText("Instances segment")).toHaveClass(
      "opacity-25",
    );
    fireEvent.mouseLeave(
      screen.getByRole("button", { name: "Highlight Java: 5 GB" }),
    );
    expect(screen.getByLabelText("Java segment")).toHaveClass("opacity-100");
    expect(screen.getByLabelText("Instances segment")).toHaveClass(
      "opacity-100",
    );
  });

  it("raises and highlights a segment when the pointer enters the bar itself", () => {
    render(
      <SegmentedBar
        label="Storage"
        segments={[
          {
            label: "Instances",
            value: 2,
            valueLabel: "2 GB",
            color: "primary",
          },
          { label: "Java", value: 5, valueLabel: "5 GB", color: "violet" },
        ]}
      />,
    );
    const java = screen.getByLabelText("Java segment");
    const highlight = screen.getByLabelText("Storage highlight");
    expect(java).toHaveAttribute("height", "12");
    fireEvent.mouseEnter(java);
    expect(highlight).toHaveAttribute("height", "16");
    expect(java).toHaveClass("opacity-100");
    expect(screen.getByLabelText("Instances segment")).toHaveClass(
      "opacity-25",
    );
    fireEvent.mouseLeave(java);
    expect(highlight).toHaveAttribute("height", "12");
    expect(screen.getByLabelText("Instances segment")).toHaveClass(
      "opacity-100",
    );
  });

  it("supports keyboard focus and restores the focused segment after mouse hover", () => {
    render(
      <SegmentedBar
        label="Storage"
        segments={[
          {
            label: "Instances",
            value: 2,
            valueLabel: "2 GB",
            color: "primary",
          },
          { label: "Java", value: 5, valueLabel: "5 GB", color: "violet" },
        ]}
      />,
    );
    const instances = screen.getByRole("button", {
      name: "Highlight Instances: 2 GB",
    });
    const java = screen.getByRole("button", { name: "Highlight Java: 5 GB" });
    fireEvent.focus(instances);
    expect(screen.getByLabelText("Instances segment")).toHaveClass(
      "opacity-100",
    );
    expect(screen.getByLabelText("Java segment")).toHaveClass("opacity-25");
    fireEvent.mouseEnter(java);
    expect(screen.getByLabelText("Java segment")).toHaveClass("opacity-100");
    fireEvent.mouseLeave(java);
    expect(screen.getByLabelText("Instances segment")).toHaveClass(
      "opacity-100",
    );
    expect(screen.getByLabelText("Java segment")).toHaveClass("opacity-25");
    fireEvent.blur(instances);
    expect(screen.getByLabelText("Java segment")).toHaveClass("opacity-100");
  });
});
