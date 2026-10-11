import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { ConsoleLine } from "@/features/instances/model/console-line";
import { ConsoleSection } from "@/features/instances/sections/console-section";

const lines: ConsoleLine[] = [
  { id: 1, stream: "stdout", line: "[t/INFO]: hello" },
  { id: 2, stream: "stdout", line: "[t/WARN]: careful" },
  { id: 3, stream: "stdout", line: "[t/ERROR]: boom" },
];

describe("ConsoleSection", () => {
  it("shows a mono console and filters by multi level selection", async () => {
    render(<ConsoleSection lines={lines} />);

    const log = screen.getByLabelText("Game log");
    expect(log).toBeInTheDocument();
    expect(log.querySelector("[data-slot='scroll-area']")).toHaveClass(
      "bg-black/25",
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
