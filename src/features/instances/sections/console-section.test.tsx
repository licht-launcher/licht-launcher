import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";

import type { LogLevelSelection } from "@/features/instances/lib/log-level";
import type { ConsoleLine } from "@/features/instances/model/console-line";
import {
  ConsoleLogLevelFilter,
  ConsoleSection,
} from "@/features/instances/sections/console-section";

const lines: ConsoleLine[] = [
  { id: 1, stream: "stdout", line: "[t/INFO]: hello" },
  { id: 2, stream: "stdout", line: "[t/WARN]: careful" },
  { id: 3, stream: "stdout", line: "[t/ERROR]: boom" },
];

function ConsoleHarness() {
  const [selected, setSelected] = useState<LogLevelSelection>(() => new Set());
  return (
    <>
      <ConsoleLogLevelFilter
        selected={selected}
        onSelectedChange={setSelected}
      />
      <ConsoleSection lines={lines} selected={selected} />
    </>
  );
}

describe("ConsoleSection", () => {
  it("shows a mono console and filters by multi level selection", async () => {
    render(<ConsoleHarness />);

    const log = screen.getByLabelText("Game log");
    expect(log).toBeInTheDocument();
    expect(log.querySelector("[data-slot='scroll-area']")).toHaveClass(
      "border-border/60",
      "bg-black/20",
    );
    expect(log.querySelector("pre")).toHaveClass("font-mono");
    expect(log.querySelector("pre")).toHaveClass("select-text");
    expect(log.querySelector("pre")).toHaveClass("cursor-text");
    expect(log.querySelector("pre")?.className).toMatch(
      /selection:bg-muted-foreground/,
    );
    expect(log).toHaveTextContent("[t/INFO]: hello");
    expect(log).toHaveTextContent("[t/WARN]: careful");
    expect(log).toHaveTextContent("[t/ERROR]: boom");
    expect(screen.getByText("INFO")).toHaveClass("text-info");
    expect(screen.getByText("WARN")).toHaveClass("text-warning");
    expect(screen.getByText("ERROR")).toHaveClass("text-destructive");

    fireEvent.click(screen.getByRole("button", { name: "Log level" }));
    expect(await screen.findByText("Log Level")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Warn" }));
    expect(screen.getByRole("button", { name: "Log level" })).toHaveTextContent(
      "Warn",
    );
    expect(log).not.toHaveTextContent("hello");
    expect(log).toHaveTextContent("careful");
    expect(log).not.toHaveTextContent("boom");

    fireEvent.click(screen.getByRole("button", { name: "Error" }));
    expect(screen.getByRole("button", { name: "Log level" })).toHaveTextContent(
      "Warn + Error",
    );
    expect(log).not.toHaveTextContent("hello");
    expect(log).toHaveTextContent("careful");
    expect(log).toHaveTextContent("boom");
  });
});
