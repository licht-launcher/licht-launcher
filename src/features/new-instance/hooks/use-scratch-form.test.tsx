import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { listLoaderBuilds, listVersions } from "@/api/commands";
import { useScratchForm } from "@/features/new-instance/hooks/use-scratch-form";
import type { LoaderBuild } from "@/lib/generated/LoaderBuild";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

vi.mock("@/api/commands", () => ({
  listVersions: vi.fn(),
  listLoaderBuilds: vi.fn(),
}));

const versions: VersionSummary[] = [
  { id: "24w14a", versionType: "snapshot", installed: false },
  { id: "1.20.1", versionType: "release", installed: false },
  { id: "1.19.4", versionType: "release", installed: false },
];

const builds: LoaderBuild[] = [
  { version: "0.16.0", stable: true },
  { version: "0.15.11", stable: true },
];

function setup() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  return renderHook(() => useScratchForm(), { wrapper });
}

beforeEach(() => {
  vi.mocked(listVersions).mockReset().mockResolvedValue(versions);
  vi.mocked(listLoaderBuilds).mockReset().mockResolvedValue(builds);
});

describe("useScratchForm", () => {
  it("starts loading the game versions as soon as it mounts", async () => {
    const { result } = setup();

    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );
    expect(listVersions).toHaveBeenCalledOnce();
  });

  it("selects the newest release, skipping snapshots", async () => {
    const { result } = setup();

    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );
  });

  it("merges patches without dropping the other fields", async () => {
    const { result } = setup();

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
    const { result } = setup();
    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );

    act(() => {
      result.current.update({ platform: "fabric" });
    });
    await waitFor(() =>
      expect(result.current.state.loaderVersion).toBe("0.16.0"),
    );
    expect(listLoaderBuilds).toHaveBeenCalledWith("fabric", "1.20.1");
  });

  it("keeps a loader version chosen by hand", async () => {
    const { result } = setup();
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
    await waitFor(() => expect(listLoaderBuilds).toHaveBeenCalled());
    expect(result.current.state.loaderVersion).toBe("0.15.11");
  });

  it("selects the newest loader version when switching to manual", async () => {
    const { result } = setup();
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
      expect(result.current.state.loaderVersion).toBe("0.16.0"),
    );
  });

  it("reports whether the loader has a build for the game version", async () => {
    const { result } = setup();
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
    vi.mocked(listLoaderBuilds).mockResolvedValue([]);
    const { result } = setup();
    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );

    act(() => {
      result.current.update({ platform: "quilt" });
    });
    await waitFor(() => expect(result.current.loaderState).toBe("empty"));
  });

  it("flags a loader whose builds could not be checked", async () => {
    vi.mocked(listLoaderBuilds).mockRejectedValue(new Error("404"));
    const { result } = setup();
    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );

    act(() => {
      result.current.update({ platform: "quilt" });
    });
    await waitFor(() => expect(result.current.loaderState).toBe("error"));
  });

  it("uses the latest channel when the loader has no stable build", async () => {
    vi.mocked(listLoaderBuilds).mockResolvedValue([
      { version: "0.31.0-beta.5", stable: false },
      { version: "0.31.0-beta.4", stable: false },
    ]);
    const { result } = setup();
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
    const { result } = setup();
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
    vi.mocked(listLoaderBuilds).mockRejectedValue(new Error("offline"));
    const { result } = setup();
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

  it("follows the newest stable build on stable and the newest on latest", async () => {
    vi.mocked(listLoaderBuilds).mockResolvedValue([
      { version: "0.17.0-beta.1", stable: false },
      { version: "0.16.0", stable: true },
    ]);
    const { result } = setup();
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
      result.current.update({ loaderChannel: "latest" });
    });
    await waitFor(() =>
      expect(result.current.state.loaderVersion).toBe("0.17.0-beta.1"),
    );
  });

  it("asks for the builds once whichever channel is picked", async () => {
    const { result } = setup();
    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );

    act(() => {
      result.current.update({ platform: "fabric" });
    });
    await waitFor(() => expect(result.current.loaderState).toBe("ready"));
    act(() => {
      result.current.update({ loaderChannel: "latest" });
    });
    act(() => {
      result.current.update({ loaderChannel: "manual", loaderVersion: "" });
    });
    await waitFor(() => expect(result.current.loaderState).toBe("ready"));

    expect(listLoaderBuilds).toHaveBeenCalledOnce();
  });

  it("does not ask for loader versions on vanilla", async () => {
    const { result } = setup();
    await waitFor(() =>
      expect(result.current.state.gameVersion).toBe("1.20.1"),
    );

    expect(listLoaderBuilds).not.toHaveBeenCalled();
  });
});
