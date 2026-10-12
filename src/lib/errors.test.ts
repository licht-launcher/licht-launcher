import { describe, expect, it } from "vitest";

import { friendlyError, messageOf } from "@/lib/errors";

describe("friendly errors", () => {
  it("turns a network failure into a short sentence", () => {
    expect(friendlyError("HTTP request failed")).toBe(
      "The network request failed. Check your connection and try again.",
    );
  });

  it("turns a disk failure into a short sentence", () => {
    expect(friendlyError("I/O operation failed")).toBe(
      "The disk could not be written. Free some space and try again.",
    );
  });

  it("turns a damaged download into a short sentence without the hash", () => {
    expect(friendlyError("downloaded SHA1 does not match")).toBe(
      "A downloaded file was damaged. Try again.",
    );
    expect(friendlyError("downloaded SHA512 does not match")).toBe(
      "A downloaded file was damaged. Try again.",
    );
    expect(friendlyError("downloaded SHA1 does not match")).not.toMatch(
      /[0-9a-f]{40}/,
    );
  });

  it("turns Microsoft auth failures into short sentences", () => {
    expect(
      friendlyError(
        "Minecraft services rejected this app (not allow-listed yet)",
      ),
    ).toMatch(/Mojang approval/);
    expect(
      friendlyError(
        "this Microsoft account does not own Minecraft: Java Edition",
      ),
    ).toMatch(/does not own/);
  });

  it("turns a missing version into a short sentence", () => {
    expect(friendlyError("version was not found")).toMatch(/missing/i);
  });

  it("lists curseforge mods that disallow third-party distribution", () => {
    const message = friendlyError(
      "curseforge mods disallow third-party distribution: " +
        "Iris (iris.jar), Sodium (sodium.jar)",
    );
    expect(message).toMatch(/Iris \(iris\.jar\)/);
    expect(message).toMatch(/Sodium \(sodium\.jar\)/);
    expect(message).toMatch(/not installed/);
  });

  it("names a curseforge file that the api no longer returns", () => {
    expect(friendlyError("curseforge file was not found: 42")).toMatch(/42/);
  });

  it("keeps an unknown message", () => {
    expect(friendlyError("something totally unknown")).toBe(
      "something totally unknown",
    );
  });

  it("reads a string or an Error", () => {
    expect(messageOf("HTTP request failed")).toBe("HTTP request failed");
    expect(messageOf(new Error("disk"))).toBe("disk");
    expect(messageOf(null)).toBe("Something went wrong.");
  });
});
