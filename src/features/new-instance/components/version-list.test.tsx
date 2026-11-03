import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { VersionList } from "@/features/new-instance/components/version-list";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

const versions: VersionSummary[] = [
  { id: "1.21", versionType: "release", installed: false },
  { id: "24w14a", versionType: "snapshot", installed: false },
  { id: "1.20.1", versionType: "release", installed: true },
];

beforeEach(() => {
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(
    function (this: HTMLElement) {
      return this.dataset.slot === "version-viewport" ? 288 : 0;
    },
  );
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(400);
});

afterEach(() => {
  vi.restoreAllMocks();
});

function Harness({
  list = versions,
  initial = "",
  pending = false,
  error = false,
  onRetry = () => {},
  onSelect = () => {},
}: {
  list?: VersionSummary[];
  initial?: string;
  pending?: boolean;
  error?: boolean;
  onRetry?: () => void;
  onSelect?: (id: string) => void;
}) {
  const [selected, setSelected] = useState(initial);
  const [includeSnapshots, setIncludeSnapshots] = useState(false);
  return (
    <VersionList
      versions={list}
      selected={selected}
      pending={pending}
      error={error}
      includeSnapshots={includeSnapshots}
      onSelect={(id) => {
        setSelected(id);
        onSelect(id);
      }}
      onIncludeSnapshotsChange={setIncludeSnapshots}
      onRetry={onRetry}
    />
  );
}

describe("VersionList", () => {
  it("lists releases only until snapshots are requested", () => {
    render(<Harness />);

    expect(screen.getByRole("button", { name: "1.21" })).toBeVisible();
    expect(screen.getByRole("button", { name: "1.20.1" })).toBeVisible();
    expect(screen.queryByRole("button", { name: "24w14a" })).toBeNull();
    fireEvent.click(screen.getByRole("switch", { name: "Include snapshots" }));
    expect(screen.getByRole("button", { name: "24w14a" })).toBeVisible();
  });

  it("filters by text and recovers from an empty search", () => {
    render(<Harness />);

    const search = screen.getByLabelText("Search versions");
    fireEvent.change(search, { target: { value: "nope" } });
    expect(screen.getByText("No versions match.")).toBeVisible();
    fireEvent.change(search, { target: { value: "1.20" } });
    expect(screen.getByRole("button", { name: "1.20.1" })).toBeVisible();
    expect(screen.queryByRole("button", { name: "1.21" })).toBeNull();
  });

  it("reports the picked version and marks it", () => {
    const onSelect = vi.fn();
    render(<Harness initial="1.21" onSelect={onSelect} />);

    expect(screen.getByRole("button", { name: "1.21" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    fireEvent.click(screen.getByRole("button", { name: "1.20.1" }));
    expect(onSelect).toHaveBeenCalledWith("1.20.1");
    expect(screen.getByRole("button", { name: "1.20.1" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("only renders the rows that fit instead of the whole manifest", () => {
    const many: VersionSummary[] = Array.from({ length: 600 }, (_, index) => ({
      id: `1.${index}`,
      versionType: "release" as const,
      installed: false,
    }));
    render(<Harness list={many} />);

    const rows = screen.getAllByRole("button", { name: /^1\./ });
    expect(rows.length).toBeGreaterThan(0);
    expect(rows.length).toBeLessThan(60);
  });

  it("shows loading and lets the user retry after an error", () => {
    const onRetry = vi.fn();
    const { rerender } = render(<Harness pending />);
    expect(screen.getByText("Loading versions…")).toBeVisible();

    rerender(<Harness error list={[]} onRetry={onRetry} />);
    expect(screen.getByText(/Could not load the version list/)).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(onRetry).toHaveBeenCalledOnce();
  });
});
