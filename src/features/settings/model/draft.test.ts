import { describe, expect, it } from "vitest";
import { snapshot } from "@/features/settings/fixtures/snapshot";
import {
  fromSnapshot,
  optionalSize,
  toSettings,
} from "@/features/settings/model/draft";

describe("draft", () => {
  it("fills empty java fields from the runtime paths", () => {
    const draft = fromSnapshot(snapshot, {
      java25: null,
      java21: "C:\\runtime\\java.exe",
      java17: null,
      java8: null,
    });

    expect(draft.java21).toBe("C:\\runtime\\java.exe");
    expect(draft.java25).toBe("");
    expect(draft.width).toBe("1280");
    expect(draft.height).toBe("720");
  });

  it("keeps a saved java path over the runtime path", () => {
    const draft = fromSnapshot(
      {
        ...snapshot,
        settings: {
          ...snapshot.settings,
          java21: "C:\\saved\\java.exe",
        },
      },
      {
        java25: null,
        java21: "C:\\runtime\\java.exe",
        java17: null,
        java8: null,
      },
    );

    expect(draft.java21).toBe("C:\\saved\\java.exe");
  });

  it("rejects a window size below one", () => {
    expect(optionalSize("0")).toBe("invalid");
    expect(optionalSize("")).toBeNull();
    expect(optionalSize("854")).toBe(854);
  });

  it("turns the draft into settings", () => {
    const draft = fromSnapshot(snapshot, null);
    const settings = toSettings(draft, 1, 854, 480);

    expect(settings).toMatchObject({
      schema: 1,
      theme: "dark",
      username: "Steve",
      width: 854,
      height: 480,
      downloadConcurrency: 8,
      writeConcurrency: 8,
      dataDirectory: snapshot.applicationDirectory,
    });
  });
});
