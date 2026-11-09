import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@/features/settings/fixtures/invoke";
import {
  mockSettingsCommands,
  renderSettings,
  snapshot,
} from "@/features/settings/fixtures/render-settings";

vi.mock("@tauri-apps/api/core", async () => {
  const { invoke: mocked } = await import(
    "@/features/settings/fixtures/invoke"
  );
  return { invoke: mocked };
});

describe("JavaSection", () => {
  beforeEach(() => {
    mockSettingsCommands();
  });

  it("keeps the configured path unchanged when a managed Java download is cancelled", async () => {
    mockSettingsCommands((command) =>
      command === "install_recommended_java"
        ? Promise.resolve(null)
        : undefined,
    );
    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Runtime" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Recommended install for Java 25" }),
    );
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("install_recommended_java", {
        major: 25,
      }),
    );
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Recommended install for Java 25" }),
      ).toBeEnabled(),
    );
    expect(screen.getByLabelText("Java 25")).toHaveValue("");
    expect(invoke).not.toHaveBeenCalledWith("save_settings", expect.anything());
  });

  it("uses an installed runtime java and leaves the others empty", async () => {
    mockSettingsCommands((command) => {
      if (command === "get_settings") {
        return Promise.resolve({
          ...snapshot,
          settings: {
            ...snapshot.settings,
            java21: "C:\\saved\\java.exe",
          },
        });
      }
      if (command === "runtime_java") {
        return Promise.resolve({
          java25: null,
          java21: "C:\\runtime\\java.exe",
          java17: "C:\\runtime\\java17.exe",
          java8: null,
        });
      }
      if (command === "java_installation_status") {
        return Promise.resolve({
          java25: false,
          java21: true,
          java17: true,
          java8: false,
        });
      }
      return undefined;
    });

    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Runtime" }));

    expect(screen.getByLabelText("Java 21")).toHaveValue("C:\\saved\\java.exe");
    expect(screen.getByLabelText("Java 17")).toHaveValue(
      "C:\\runtime\\java17.exe",
    );
    expect(screen.getByLabelText("Java 25")).toHaveValue("");
    expect(screen.getByLabelText("Java 25")).toHaveAttribute(
      "placeholder",
      "path/to/java",
    );
    expect(
      screen.getByRole("img", { name: "Java 25 is invalid" }),
    ).toBeInTheDocument();
    expect(
      await screen.findByRole("img", { name: "Java 17 is valid" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", {
        name: "Recommended install for Java 17",
      }),
    ).toBeDisabled();
    expect(
      screen.getByRole("button", {
        name: "Recommended install for Java 25",
      }),
    ).toBeEnabled();
  });

  it("keeps the other icons and shows a loader on the path being checked", async () => {
    mockSettingsCommands((command) => {
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "runtime_java") {
        return Promise.resolve({
          java25: null,
          java21: null,
          java17: "C:\\runtime\\java17.exe",
          java8: null,
        });
      }
      if (command === "java_installation_status") {
        return new Promise(() => undefined);
      }
      if (command === "detect_java_installations") {
        return Promise.resolve({
          java25: [],
          java21: [],
          java17: [],
          java8: [],
        });
      }
      return undefined;
    });

    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Runtime" }));

    expect(
      screen.getByRole("img", { name: "Java 25 is invalid" }),
    ).toBeInTheDocument();
    expect(
      await screen.findByRole("img", { name: "Checking Java 17" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", {
        name: "Recommended install for Java 17",
      }),
    ).toBeDisabled();
    expect(
      screen.getByRole("button", {
        name: "Recommended install for Java 25",
      }),
    ).toBeEnabled();
    expect(
      screen.getByRole("img", { name: "Java 25 is invalid" }),
    ).toBeInTheDocument();
  });

  it("opens detected javas in a table and marks the current path as selected", async () => {
    const selected =
      "C:\\Users\\Mateus\\AppData\\Roaming\\ModrinthApp\\meta\\java_versions\\zulu21.48.17-ca-jre21.0.10-win_x64\\bin\\javaw.exe";
    const runtime = "C:\\Licht\\runtime\\temurin21\\bin\\java.exe";
    mockSettingsCommands((command) => {
      if (command === "get_settings") {
        return Promise.resolve({
          ...snapshot,
          settings: { ...snapshot.settings, java21: selected },
        });
      }
      if (command === "runtime_java") {
        return Promise.resolve({
          java25: null,
          java21: runtime,
          java17: null,
          java8: null,
        });
      }
      if (command === "java_installation_status") {
        return Promise.resolve({
          java25: false,
          java21: false,
          java17: false,
          java8: false,
        });
      }
      if (command === "detect_java_installations") {
        return Promise.resolve({
          java25: [],
          java21: [selected],
          java17: [],
          java8: [],
        });
      }
      return undefined;
    });

    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Runtime" }));
    expect(
      await screen.findByRole("img", { name: "Java 21 is invalid" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", {
        name: "Recommended install for Java 21",
      }),
    ).toBeEnabled();
    const detect = await screen.findByRole("button", {
      name: "Detect installations for Java 21",
    });
    await waitFor(() => {
      expect(detect).toBeEnabled();
    });
    fireEvent.click(detect);

    expect(screen.getByRole("columnheader", { name: "Versão" })).toBeVisible();
    expect(screen.getByRole("columnheader", { name: "Caminho" })).toBeVisible();
    expect(screen.getByRole("columnheader", { name: "Ações" })).toBeVisible();
    expect(screen.getAllByRole("cell", { name: "21" })).toHaveLength(2);
    expect(screen.getByRole("cell", { name: selected })).toBeVisible();
    expect(screen.getByRole("cell", { name: runtime })).toBeVisible();
    expect(screen.getByRole("button", { name: "Selecionado" })).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "Selecionar" }));

    expect(
      screen.getByRole("row", { name: `21 ${runtime} Selecionado` }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Selecionado" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(screen.getByLabelText("Java 21")).toHaveValue(runtime);
  });

  it("does not check a java path again when the section is reopened", async () => {
    let checks = 0;
    mockSettingsCommands((command) => {
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "runtime_java") {
        return Promise.resolve({
          java25: null,
          java21: null,
          java17: "C:\\runtime\\java17.exe",
          java8: null,
        });
      }
      if (command === "java_installation_status") {
        checks += 1;
        return Promise.resolve({
          java25: false,
          java21: false,
          java17: true,
          java8: false,
        });
      }
      if (command === "detect_java_installations") {
        return Promise.resolve({
          java25: [],
          java21: [],
          java17: [],
          java8: [],
        });
      }
      return undefined;
    });

    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Runtime" }));
    expect(
      await screen.findByRole("img", { name: "Java 17 is valid" }),
    ).toBeInTheDocument();
    const first = checks;
    fireEvent.click(screen.getByRole("button", { name: "Instances" }));
    fireEvent.click(screen.getByRole("button", { name: "Runtime" }));
    expect(
      screen.getByRole("img", { name: "Java 17 is valid" }),
    ).toBeInTheDocument();
    expect(checks).toBe(first);
  });

  it("disables detect when no Java installation was found", async () => {
    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Runtime" }));

    const detect = await screen.findByRole("button", {
      name: "Detect installations for Java 25",
    });
    expect(detect).toBeDisabled();
  });

  it("shows java arguments after the paths", async () => {
    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Runtime" }));

    expect(screen.getByLabelText("Java arguments")).toHaveAttribute(
      "placeholder",
      "-XX:+UseG1GC",
    );
    expect(
      screen.getByText("Extra JVM flags, separated by spaces."),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Java paths and extra JVM arguments for launch."),
    ).toBeInTheDocument();
  });
});
