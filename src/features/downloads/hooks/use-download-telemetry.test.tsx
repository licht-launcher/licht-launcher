import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  DOWNLOAD_SPEED_HISTORY_QUERY_KEY,
  DOWNLOAD_TELEMETRY_STATUS_QUERY_KEY,
  type SpeedHistory,
  useDownloadTelemetry,
} from "@/features/downloads/hooks/use-download-telemetry";
import { useDownloadTelemetrySync } from "@/features/downloads/hooks/use-download-telemetry-sync";
import type { DownloadTelemetry } from "@/lib/generated/DownloadTelemetry";

const { invoke, listen, dispose } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  dispose: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

let emit: ((event: { payload: DownloadTelemetry }) => void) | undefined;
const initial: DownloadTelemetry = {
  receivedBytes: 0,
  writtenBytes: 0,
  paused: false,
};

function Collector({ active }: { active: boolean }) {
  useDownloadTelemetrySync(active);
  return null;
}

function Viewer() {
  const query = useDownloadTelemetry();
  return (
    <output aria-label="Speed history">{JSON.stringify(query.samples)}</output>
  );
}

function view(client: QueryClient, active: boolean, visible: boolean) {
  return (
    <QueryClientProvider client={client}>
      <Collector active={active} />
      {visible && <Viewer />}
    </QueryClientProvider>
  );
}

async function start(active = true) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const rendered = render(view(client, active, true));
  await act(async () => vi.advanceTimersByTimeAsync(0));
  act(() => emit?.({ payload: initial }));
  return { client, ...rendered };
}

function history(client: QueryClient): SpeedHistory {
  const recorded = client.getQueryData<SpeedHistory>(
    DOWNLOAD_SPEED_HISTORY_QUERY_KEY,
  );
  if (!recorded) throw new Error("History cache was not initialized");
  return recorded;
}

async function advance(milliseconds: number) {
  await act(async () => vi.advanceTimersByTimeAsync(milliseconds));
}

function receive(receivedBytes: number, writtenBytes = receivedBytes / 2) {
  act(() => emit?.({ payload: { ...initial, receivedBytes, writtenBytes } }));
}

describe("download telemetry sampling", () => {
  beforeEach(() => {
    vi.useFakeTimers({
      now: 0,
      toFake: [
        "setTimeout",
        "clearTimeout",
        "setInterval",
        "clearInterval",
        "Date",
        "performance",
      ],
    });
    invoke.mockReset().mockResolvedValue(initial);
    listen
      .mockReset()
      .mockImplementation(
        async (
          _name: string,
          callback: (event: { payload: DownloadTelemetry }) => void,
        ) => {
          emit = callback;
          return dispose;
        },
      );
    dispose.mockReset();
    emit = undefined;
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it("closes one immutable value per second instead of updating on every event", async () => {
    const { client } = await start();
    await advance(999);
    receive(2_000_000, 500_000);
    receive(1_000_000, 250_000);
    expect(history(client).received).toEqual([]);
    await advance(1);
    expect(history(client).received).toEqual([
      { at: 1000, bytesPerSecond: 2_000_000 },
    ]);
    expect(history(client).written).toEqual([
      { at: 1000, bytesPerSecond: 500_000 },
    ]);
    const closed = history(client).received[0];
    receive(3_000_000, 1_000_000);
    await advance(999);
    expect(history(client).received).toEqual([closed]);
    await advance(1);
    expect(history(client).received).toEqual([
      closed,
      { at: 2000, bytesPerSecond: 1_000_000 },
    ]);
    expect(history(client).received[0]).toBe(closed);
  });

  it("continues collecting with the viewer unmounted and restores its cached history", async () => {
    const { client, rerender } = await start();
    receive(1_000_000);
    await advance(1000);
    rerender(view(client, true, false));
    expect(screen.queryByLabelText("Speed history")).not.toBeInTheDocument();
    receive(3_000_000);
    await advance(1000);
    expect(history(client).received).toEqual([
      { at: 1000, bytesPerSecond: 1_000_000 },
      { at: 2000, bytesPerSecond: 2_000_000 },
    ]);
    rerender(view(client, true, true));
    await advance(0);
    expect(screen.getByLabelText("Speed history")).toHaveTextContent(
      '"bytesPerSecond":2000000',
    );
    receive(4_000_000);
    await advance(1000);
    expect(history(client).received.at(-1)).toEqual({
      at: 3000,
      bytesPerSecond: 1_000_000,
    });
    expect(listen).toHaveBeenCalledOnce();
  });

  it("does not inflate the first partial second into an instantaneous speed spike", async () => {
    const { client, rerender } = await start(false);
    await advance(900);
    rerender(view(client, true, true));
    receive(100_000);
    await advance(100);
    expect(history(client).received.at(-1)).toEqual({
      at: 1000,
      bytesPerSecond: 100_000,
    });
  });

  it("freezes history while inactive and resumes without charging paused time as traffic", async () => {
    const { client, rerender } = await start();
    receive(1_000_000);
    await advance(1000);
    const frozen = history(client);
    rerender(view(client, false, true));
    receive(5_000_000);
    await advance(5000);
    expect(history(client)).toBe(frozen);
    rerender(view(client, true, true));
    receive(6_000_000);
    await advance(1000);
    expect(history(client).received.at(-1)).toEqual({
      at: 7000,
      bytesPerSecond: 1_000_000,
    });
  });

  it("retains exactly the latest thirty completed seconds", async () => {
    const { client } = await start();
    for (let second = 1; second <= 34; second += 1) {
      receive(second * 1_000_000);
      await advance(1000);
    }
    expect(history(client).received).toHaveLength(30);
    expect(history(client).received[0]?.at).toBe(5000);
    expect(history(client).received.at(-1)?.at).toBe(34_000);
    expect(
      history(client).received.every(
        (point) => point.bytesPerSecond === 1_000_000,
      ),
    ).toBe(true);
  });

  it("preserves real history instead of recording zeros when fallback telemetry fails", async () => {
    const { client } = await start();
    receive(1_000_000);
    await advance(1000);
    const frozen = history(client);
    invoke.mockRejectedValue(new Error("Telemetry unavailable"));
    act(() => client.setQueryData(DOWNLOAD_TELEMETRY_STATUS_QUERY_KEY, true));
    await advance(0);
    await advance(3000);
    expect(history(client)).toBe(frozen);
  });
});
