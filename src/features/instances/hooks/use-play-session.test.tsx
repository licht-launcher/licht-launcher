import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  completedDownload,
  downloadJob,
} from "@/features/downloads/fixtures/job";
import { usePlaySession } from "@/features/instances/hooks/use-play-session";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

const { state, readiness, prepare, launch, running, touch } = vi.hoisted(() => {
  const state: { jobs: DownloadJobSnapshot[] } = { jobs: [] };
  return {
    state,
    readiness: vi.fn(),
    prepare: vi.fn(),
    launch: vi.fn(),
    running: vi.fn(),
    touch: vi.fn(),
  };
});
vi.mock("@/features/downloads/hooks/use-download-jobs", () => ({
  useDownloadJobs: () => ({ data: state.jobs }),
}));
vi.mock("@/lib/commands", () => ({
  isVersionReady: readiness,
  prepareInstanceDownload: prepare,
  launchVersion: launch,
  listRunningGames: running,
  listVersions: vi.fn(),
  touchLastOpened: touch,
  stopGame: vi.fn(),
}));
vi.mock("sonner", () => ({ toast: { error: vi.fn() } }));

const instance: InstanceEntry = {
  folder: "Survival",
  name: "Survival",
  versionId: "1.20.1",
  minMemoryMb: 512,
  maxMemoryMb: 2048,
  jvmArguments: [],
  fullscreen: false,
  width: null,
  height: null,
  overrideWindow: false,
  overrideMemory: false,
  overrideJava: false,
  overrideJvmArguments: false,
  javaPath: null,
  lastOpenedAt: null,
  iconPath: null,
};

function session() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return renderHook(
    () =>
      usePlaySession({
        username: "Steve",
        clearVersionProgress: vi.fn(),
        clearFolderLog: vi.fn(),
      }),
    {
      wrapper: ({ children }: { children: ReactNode }) => (
        <QueryClientProvider client={client}>{children}</QueryClientProvider>
      ),
    },
  );
}

describe("Play readiness", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    state.jobs = [];
    running.mockResolvedValue([]);
    readiness.mockResolvedValue(true);
    prepare.mockResolvedValue(
      completedDownload({ instanceFolder: instance.folder }),
    );
    launch.mockResolvedValue({ code: 0 });
    touch.mockResolvedValue(instance);
  });

  it("prepares a resolved but incomplete installation without consulting readiness", async () => {
    const { result } = session();
    const pending: InstanceEntry = {
      ...instance,
      installation: {
        state: "installing",
        source: { type: "vanilla", gameVersion: "1.20.1" },
      },
    };
    await act(async () => {
      await result.current.playInstance(pending);
    });
    expect(prepare).toHaveBeenCalledWith(instance.folder);
    expect(readiness).not.toHaveBeenCalled();
    expect(launch).not.toHaveBeenCalled();
  });

  it.each(["paused", "interrupted", "failed"] as const)(
    "prepares an existing %s job without consulting readiness",
    async (status) => {
      state.jobs = [downloadJob({ status, instanceFolder: instance.folder })];
      const { result } = session();
      await act(async () => {
        await result.current.playInstance(instance);
      });
      expect(prepare).toHaveBeenCalledWith(instance.folder);
      expect(readiness).not.toHaveBeenCalled();
      expect(launch).toHaveBeenCalledWith(instance.folder, "Steve");
    },
  );

  it("checks a ready instance once and launches without preparation", async () => {
    const { result } = session();
    await act(async () => {
      await result.current.playInstance(instance);
    });
    expect(readiness).toHaveBeenCalledExactlyOnceWith(instance.versionId);
    expect(prepare).not.toHaveBeenCalled();
    expect(launch).toHaveBeenCalledWith(instance.folder, "Steve");
  });

  it("prepares missing prerequisites before launching", async () => {
    readiness.mockResolvedValue(false);
    const { result } = session();
    await act(async () => {
      await result.current.playInstance(instance);
    });
    expect(readiness).toHaveBeenCalledExactlyOnceWith(instance.versionId);
    expect(prepare).toHaveBeenCalledWith(instance.folder);
    expect(launch).toHaveBeenCalledWith(instance.folder, "Steve");
  });
});
