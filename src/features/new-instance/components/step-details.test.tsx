import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";

import { StepDetails } from "@/features/new-instance/components/step-details";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

const versions: VersionSummary[] = [
  { id: "1.20.1", versionType: "release", installed: false },
  { id: "24w14a", versionType: "snapshot", installed: false },
];

function Details() {
  const [includeSnapshots, setIncludeSnapshots] = useState(false);
  const [gameVersion, setGameVersion] = useState("1.20.1");
  return (
    <StepDetails
      platform="vanilla"
      name="Survival"
      gameVersion={gameVersion}
      includeSnapshots={includeSnapshots}
      versions={versions}
      versionsPending={false}
      versionsError={false}
      onRetryVersions={() => {}}
      loaderChannel="stable"
      loaderVersion=""
      loaderVersions={[]}
      loaderVersionsPending={false}
      onNameChange={() => {}}
      onGameVersionChange={setGameVersion}
      onIncludeSnapshotsChange={setIncludeSnapshots}
      onLoaderChannelChange={() => {}}
      onLoaderVersionChange={() => {}}
    />
  );
}

describe("StepDetails version picker", () => {
  it("includes snapshots when requested", async () => {
    render(<Details />);
    expect(screen.queryByRole("button", { name: "24w14a" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Include snapshots" }));
    expect(await screen.findByRole("button", { name: "24w14a" })).toBeVisible();
  });

  it("filters versions and recovers from an empty search", () => {
    render(<Details />);
    const search = screen.getByLabelText("Search versions");
    fireEvent.change(search, { target: { value: "nope" } });
    expect(screen.getByText("No versions match.")).toBeVisible();
    fireEvent.change(search, { target: { value: "1.20" } });
    expect(screen.getByRole("button", { name: "1.20.1" })).toBeVisible();
  });
});
