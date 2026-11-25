import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useInstanceContent } from "@/features/instances/hooks/use-instance-content";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";

const { listInstanceContent, setInstanceContentEnabled } = vi.hoisted(() => ({
  listInstanceContent: vi.fn(),
  setInstanceContentEnabled: vi.fn(),
}));
vi.mock("@/api/commands", () => ({
  listInstanceContent,
  setInstanceContentEnabled,
  removeInstanceContent: vi.fn(),
}));
vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: vi.fn() } }));

const entry: InstanceContentEntry = {
  name: "sodium.jar",
  kind: "mods",
  enabled: true,
  size: 1024,
  title: "Sodium",
  version: "0.6.5",
  iconUrl: null,
  fromModpack: false,
};

function renderContent(folder: string) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  return renderHook(
    (props: { folder: string }) => useInstanceContent(props.folder),
    {
      wrapper,
      initialProps: { folder },
    },
  );
}

describe("useInstanceContent", () => {
  beforeEach(() => {
    listInstanceContent.mockReset().mockResolvedValue([entry]);
    setInstanceContentEnabled.mockReset();
  });

  it("reloads the instance it changed, even after another is selected", async () => {
    let finishToggle: () => void = () => {};
    setInstanceContentEnabled.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          finishToggle = resolve;
        }),
    );
    const view = renderContent("Alpha");
    await waitFor(() =>
      expect(view.result.current.content.isSuccess).toBe(true),
    );

    act(() => view.result.current.toggle(entry, false));
    view.rerender({ folder: "Beta" });
    await waitFor(() =>
      expect(listInstanceContent).toHaveBeenCalledWith(
        "Beta",
        false,
        expect.any(Function),
        expect.any(AbortSignal),
      ),
    );
    listInstanceContent.mockClear();
    await act(async () => finishToggle());

    expect(setInstanceContentEnabled).toHaveBeenCalledWith(
      "Alpha",
      "mods",
      "sodium.jar",
      false,
    );
    // Alpha's list waits unused until it is selected again, then reads afresh.
    await waitFor(() => expect(view.result.current.busy).toBe(false));
    view.rerender({ folder: "Alpha" });
    await waitFor(() =>
      expect(listInstanceContent).toHaveBeenCalledWith(
        "Alpha",
        false,
        expect.any(Function),
        expect.any(AbortSignal),
      ),
    );
  });
});
