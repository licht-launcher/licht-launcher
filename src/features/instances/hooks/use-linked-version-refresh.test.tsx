import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useLinkedVersionRefresh } from "@/features/instances/hooks/use-linked-version-refresh";
import type { ModpackOrigin } from "@/features/instances/lib/modpack-content";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";
import type { LinkedModpack } from "@/lib/generated/LinkedModpack";

const { refreshLinkedModpackVersion } = vi.hoisted(() => ({
  refreshLinkedModpackVersion: vi.fn(),
}));
vi.mock("@/api/commands", () => ({ refreshLinkedModpackVersion }));

const store: ModpackOrigin = {
  type: "modpack",
  source: "modrinth",
  projectId: "homestead",
  versionId: "v3",
  iconUrl: null,
};

const linked: LinkedModpack = {
  name: "Homestead",
  author: null,
  versionName: "Homestead 2.0.1",
  publishedAt: null,
};

type Props = {
  folder: string | null;
  origin: ModpackOrigin | null;
  linked: LinkedModpack | null;
};

function renderRefresh(initialProps: Props) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const invalidate = vi.spyOn(client, "invalidateQueries");
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  const view = renderHook(
    (props: Props) =>
      useLinkedVersionRefresh(props.folder, props.origin, props.linked),
    { wrapper, initialProps },
  );
  return { ...view, invalidate };
}

describe("useLinkedVersionRefresh", () => {
  beforeEach(() => {
    refreshLinkedModpackVersion.mockReset().mockResolvedValue(null);
  });

  it("asks once per instance and reloads instances when the name changed", async () => {
    refreshLinkedModpackVersion.mockResolvedValue({} as InstanceEntry);
    const { rerender, invalidate } = renderRefresh({
      folder: "homestead",
      origin: store,
      linked,
    });

    await waitFor(() =>
      expect(invalidate).toHaveBeenCalledWith({ queryKey: ["instances"] }),
    );
    rerender({
      folder: "homestead",
      origin: store,
      linked: { ...linked, versionName: "2.0.1" },
    });
    expect(refreshLinkedModpackVersion).toHaveBeenCalledTimes(1);
    expect(refreshLinkedModpackVersion).toHaveBeenCalledWith("homestead");
  });

  it("leaves instances alone when nothing changed", async () => {
    const { invalidate } = renderRefresh({
      folder: "homestead",
      origin: store,
      linked,
    });

    await waitFor(() =>
      expect(refreshLinkedModpackVersion).toHaveBeenCalledTimes(1),
    );
    expect(invalidate).not.toHaveBeenCalled();
  });

  it("skips packs from a file and packs without saved details", () => {
    renderRefresh({
      folder: "imported",
      origin: { type: "modpackFile", source: "modrinth" },
      linked,
    });
    renderRefresh({ folder: "older", origin: store, linked: null });
    renderRefresh({ folder: null, origin: store, linked });

    expect(refreshLinkedModpackVersion).not.toHaveBeenCalled();
  });
});
