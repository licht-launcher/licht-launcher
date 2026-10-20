import { QueryClient } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { instanceContentQuery } from "@/features/instances/lib/content-query";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";

const listInstanceContent = vi.hoisted(() => vi.fn());
vi.mock("@/lib/commands", () => ({ listInstanceContent }));

const entry: InstanceContentEntry = {
  name: "one.jar",
  kind: "mods",
  enabled: true,
  size: 3,
  title: "Known title",
  version: "1.0",
  iconUrl: null,
};
type Publish = (entries: InstanceContentEntry[], initial: boolean) => void;

describe("progressive content queries", () => {
  beforeEach(() => {
    listInstanceContent.mockReset();
  });

  it("preserves cached metadata during manual refresh and removes missing rows", async () => {
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const key = ["instance-content", "classic"];
    client.setQueryData(key, [entry, { ...entry, name: "removed.jar" }]);
    client.setQueryData(["instance-content", "other"], [entry]);
    const local = { ...entry, title: "One", version: null };
    listInstanceContent.mockImplementation(
      async (_folder, refresh, publish: Publish) => {
        expect(refresh).toBe(true);
        publish([local], true);
        expect(client.getQueryData(key)).toEqual([entry]);
        publish([{ ...entry, version: "2.0" }], false);
        expect(client.getQueryData(key)).toEqual([
          { ...entry, version: "2.0" },
        ]);
        return [{ ...entry, version: "2.0" }];
      },
    );
    await client.fetchQuery({
      ...instanceContentQuery(client, "classic", true),
      staleTime: 0,
    });
    expect(client.getQueryData(["instance-content", "other"])).toEqual([entry]);
    client.clear();
  });

  it("ignores events and the final response of a cancelled older refresh", async () => {
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const key = ["instance-content", "classic"];
    client.setQueryData(key, [entry]);
    let oldPublish: Publish | undefined;
    let finishOld: ((entries: InstanceContentEntry[]) => void) | undefined;
    listInstanceContent
      .mockImplementationOnce((_folder, _refresh, publish: Publish) => {
        oldPublish = publish;
        return new Promise<InstanceContentEntry[]>((resolve) => {
          finishOld = resolve;
        });
      })
      .mockResolvedValueOnce([{ ...entry, version: "2.0" }]);
    const first = client
      .fetchQuery({
        ...instanceContentQuery(client, "classic", true),
        staleTime: 0,
      })
      .catch(() => undefined);
    await client.cancelQueries({ queryKey: key });
    await client.fetchQuery({
      ...instanceContentQuery(client, "classic", true),
      staleTime: 0,
    });
    oldPublish?.([{ ...entry, version: "old event" }], false);
    finishOld?.([{ ...entry, version: "old result" }]);
    await first;
    expect(client.getQueryData(key)).toEqual([{ ...entry, version: "2.0" }]);
    client.clear();
  });
});
