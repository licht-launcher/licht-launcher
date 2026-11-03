import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useScratchForm } from "@/features/new-instance/hooks/use-scratch-form";
import { listLoaderVersions, listVersions } from "@/lib/commands";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

vi.mock("@/lib/commands", () => ({
  listVersions: vi.fn(),
  listLoaderVersions: vi.fn(),
}));

const versions: VersionSummary[] = [
  { id: "24w14a", versionType: "snapshot", installed: false },
  { id: "1.20.1", versionType: "release", installed: false },
  { id: "1.19.4", versionType: "release", installed: false },
];

function setup(enabled: boolean) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  return renderHook(({ on }) => useScratchForm(on), {
    wrapper,
    initialProps: { on: enabled },
  });
}

beforeEach(() => {
  vi.mocked(listVersions).mockReset().mockResolvedValue(versions);
  vi.mocked(listLoaderVersions)
    .mockReset()
    .mockResolvedValue(["0.16.0", "0.15.11"]);
});

describe("useScratchForm", () => {
  it("does not load anything until it is enabled", async () => {
    const { result, rerender } = setup(false);

    expect(listVersions).not.toHaveBeenCalled();
    expect(result.current.state.gameVersion).toBe("");
    rerender({ on: true });
    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );
    expect(listVersions).toHaveBeenCalledOnce();
  });

  it("selects the newest release, skipping snapshots", async () => {
    const { result } = setup(true);

    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );
  });

  it("merges patches without dropping the other fields", async () => {
    const { result } = setup(true);

    act(() => {
      result.current.update({ name: "Modded" });
    });
    act(() => {
      result.current.update({ platform: "forge" });
    });
    expect(result.current.state).toMatchObject({
      name: "Modded",
      platform: "forge",
    });
  });

  it("fills the newest loader version for a loader platform", async () => {
    const { result } = setup(true);
    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );

    act(() => {
      result.current.update({ platform: "fabric" });
    });
    await waitFor(() =>
      expect(result.current.state.loaderVersion).toBe("0.16.0"),
    );
    expect(listLoaderVersions).toHaveBeenCalledWith(
      "fabric",
      "1.20.1",
      "stable",
    );
  });

  it("keeps a loader version chosen by hand", async () => {
    const { result } = setup(true);
    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );

    act(() => {
      result.current.update({
        platform: "fabric",
        loaderChannel: "manual",
        loaderVersion: "0.15.11",
      });
    });
    await waitFor(() => expect(listLoaderVersions).toHaveBeenCalled());
    expect(result.current.state.loaderVersion).toBe("0.15.11");
  });

  it("selects the newest loader version when switching to manual", async () => {
    const { result } = setup(true);
    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );
    act(() => {
      result.current.update({ platform: "fabric" });
    });
    await waitFor(() =>
      expect(result.current.state.loaderVersion).toBe("0.16.0"),
    );

    act(() => {
      result.current.update({ loaderChannel: "manual", loaderVersion: "" });
    });
    await waitFor(() =>
      expect(listLoaderVersions).toHaveBeenCalledWith(
        "fabric",
        "1.20.1",
        "manual",
      ),
    );
    await waitFor(() =>
      expect(result.current.state.loaderVersion).toBe("0.16.0"),
    );
  });

  it("reports whether the loader has a build for the game version", async () => {
    const { result } = setup(true);
    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );
    expect(result.current.loaderState).toBe("none");

    act(() => {
      result.current.update({ platform: "fabric" });
    });
    await waitFor(() => expect(result.current.loaderState).toBe("ready"));
  });

  it("flags a loader without builds for the game version", async () => {
    vi.mocked(listLoaderVersions).mockResolvedValue([]);
    const { result } = setup(true);
    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );

    act(() => {
      result.current.update({ platform: "quilt" });
    });
    await waitFor(() => expect(result.current.loaderState).toBe("empty"));
  });

  it("flags a loader whose builds could not be checked", async () => {
    vi.mocked(listLoaderVersions).mockRejectedValue(new Error("404"));
    const { result } = setup(true);
    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );

    act(() => {
      result.current.update({ platform: "quilt" });
    });
    await waitFor(() => expect(result.current.loaderState).toBe("error"));
  });

  it("uses the latest channel when the loader has no stable build", async () => {
    vi.mocked(listLoaderVersions).mockImplementation(
      async (_platform, _version, channel) =>
        channel === "stable" ? [] : ["0.31.0-beta.5", "0.31.0-beta.4"],
    );
    const { result } = setup(true);
    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );

    act(() => {
      result.current.update({ platform: "quilt" });
    });
    await waitFor(() =>
      expect(result.current.state.loaderChannel).toBe("latest"),
    );
    await waitFor(() =>
      expect(result.current.state.loaderVersion).toBe("0.31.0-beta.5"),
    );
    expect(result.current.stableMissing).toBe(true);
    expect(result.current.loaderState).toBe("ready");
  });

  it("stays on stable when the loader has a stable build", async () => {
    const { result } = setup(true);
    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );

    act(() => {
      result.current.update({ platform: "fabric" });
    });
    await waitFor(() => expect(result.current.loaderState).toBe("ready"));
    expect(result.current.state.loaderChannel).toBe("stable");
    expect(result.current.stableMissing).toBe(false);
  });

  it("does not switch channel when the stable check itself fails", async () => {
    vi.mocked(listLoaderVersions).mockRejectedValue(new Error("offline"));
    const { result } = setup(true);
    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );

    act(() => {
      result.current.update({ platform: "quilt" });
    });
    await waitFor(() => expect(result.current.loaderState).toBe("error"));
    expect(result.current.state.loaderChannel).toBe("stable");
    expect(result.current.stableMissing).toBe(false);
  });

  it("does not ask for loader versions on vanilla", async () => {
    const { result } = setup(true);
    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );

    expect(listLoaderVersions).not.toHaveBeenCalled();
  });
});
