import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SpeedChart } from "@/components/ui/speed-chart";

describe("SpeedChart", () => {
  it("labels an empty chart with a zero rate and its time window", () => {
    render(
      <SpeedChart
        title="Transfer speed"
        series={[
          { label: "Download", samples: [], color: "violet" },
          { label: "Write", samples: [], color: "cool" },
        ]}
      />,
    );
    expect(
      screen.getByRole("img", {
        name: "Transfer speed. Download: 0 B/s; Write: 0 B/s. Last 30 seconds.",
      }),
    ).toBeVisible();
    expect(screen.queryByText(/Peak/)).not.toBeInTheDocument();
    expect(screen.queryByText("60 seconds ago")).not.toBeInTheDocument();
  });

  it("shows download bars and a write line while excluding expired samples", () => {
    render(
      <SpeedChart
        title="Transfer speed"
        series={[
          {
            label: "Download",
            color: "violet",
            kind: "bars",
            samples: [
              { at: 31_000, bytesPerSecond: 100_000_000 },
              { at: 61_000, bytesPerSecond: 2000 },
              { at: 62_000, bytesPerSecond: 1000 },
            ],
          },
          {
            label: "Write",
            color: "cool",
            samples: [{ at: 62_000, bytesPerSecond: 500 }],
          },
        ]}
      />,
    );
    expect(
      screen.getByRole("img", {
        name: "Transfer speed. Download: 1.0 KB/s; Write: 500 B/s. Last 30 seconds.",
      }),
    ).toBeVisible();
    const bars = screen.getByLabelText("Download bars");
    expect(bars).toHaveClass("text-accent-violet");
    expect(bars.querySelectorAll("rect")).toHaveLength(2);
    expect(bars.querySelector("rect")).toHaveAttribute("height", "140");
    const write = screen.getByLabelText("Write line");
    expect(write).toHaveClass("text-accent-cool", "opacity-50");
    expect(write).toHaveAttribute("stroke-width", "1.5");
    expect(
      write.compareDocumentPosition(bars) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  it("renders separate narrow bars for each closed second using the shared scale", () => {
    render(
      <SpeedChart
        title="Transfer speed"
        series={[
          {
            label: "Download",
            color: "violet",
            kind: "bars",
            samples: [
              { at: 1000, bytesPerSecond: 1000 },
              { at: 2000, bytesPerSecond: 3000 },
            ],
          },
          {
            label: "Write",
            color: "cool",
            samples: [{ at: 2000, bytesPerSecond: 4000 }],
          },
        ]}
      />,
    );
    const bars = screen
      .getByLabelText("Download bars")
      .querySelectorAll("rect");
    expect(bars).toHaveLength(2);
    expect(bars[0]).toHaveAttribute("height", "35");
    expect(bars[1]).toHaveAttribute("height", "105");
    const spacing =
      Number(bars[1]?.getAttribute("x")) - Number(bars[0]?.getAttribute("x"));
    expect(Number(bars[0]?.getAttribute("width"))).toBeLessThan(spacing / 2);
  });

  it("keeps previously recorded values unchanged when a new second is appended", () => {
    const download = [
      { at: 1000, bytesPerSecond: 2000 },
      { at: 2000, bytesPerSecond: 1000 },
    ];
    const write = [
      { at: 1000, bytesPerSecond: 4000 },
      { at: 2000, bytesPerSecond: 0 },
    ];
    const view = render(
      <SpeedChart
        title="Transfer speed"
        series={[
          {
            label: "Download",
            color: "violet",
            kind: "bars",
            samples: download,
          },
          {
            label: "Write",
            color: "cool",
            samples: write,
          },
        ]}
      />,
    );
    expect(
      [...screen.getByLabelText("Download bars").querySelectorAll("rect")].map(
        (bar) => bar.getAttribute("height"),
      ),
    ).toEqual(["70", "35"]);
    view.rerender(
      <SpeedChart
        title="Transfer speed"
        series={[
          {
            label: "Download",
            color: "violet",
            kind: "bars",
            samples: [...download, { at: 3000, bytesPerSecond: 3000 }],
          },
          {
            label: "Write",
            color: "cool",
            samples: [...write, { at: 3000, bytesPerSecond: 4000 }],
          },
        ]}
      />,
    );
    expect(
      [...screen.getByLabelText("Download bars").querySelectorAll("rect")].map(
        (bar) => bar.getAttribute("height"),
      ),
    ).toEqual(["70", "35", "105"]);
    expect(download).toEqual([
      { at: 1000, bytesPerSecond: 2000 },
      { at: 2000, bytesPerSecond: 1000 },
    ]);
  });
});
