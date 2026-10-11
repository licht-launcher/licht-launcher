export type InstanceVersionInfo = {
  gameVersion: string | null;
  loaderLabel: string;
};

/**
 * Best-effort parse of Licht / Mojang version ids for display.
 * Fabric/Quilt: `fabric-loader-<loader>-<game>`. Forge/NeoForge: `forge-<build>`.
 */
export function describeInstanceVersion(
  versionId: string,
): InstanceVersionInfo {
  const fabric = /^fabric-loader-(.+)-(\d+\.\d+(?:\.\d+)?)$/.exec(versionId);
  if (fabric !== null) {
    return {
      gameVersion: fabric[2],
      loaderLabel: `Fabric ${fabric[1]}`,
    };
  }
  const quilt = /^quilt-loader-(.+)-(\d+\.\d+(?:\.\d+)?)$/.exec(versionId);
  if (quilt !== null) {
    return {
      gameVersion: quilt[2],
      loaderLabel: `Quilt ${quilt[1]}`,
    };
  }
  const neoforge = /^neoforge-(.+)$/.exec(versionId);
  if (neoforge !== null) {
    return {
      gameVersion: null,
      loaderLabel: `NeoForge ${neoforge[1]}`,
    };
  }
  const forge = /^forge-(.+)$/.exec(versionId);
  if (forge !== null) {
    return {
      gameVersion: null,
      loaderLabel: `Forge ${forge[1]}`,
    };
  }
  return { gameVersion: versionId, loaderLabel: "Vanilla" };
}

export function formatInstanceVersionLine(versionId: string): string {
  const { gameVersion, loaderLabel } = describeInstanceVersion(versionId);
  if (gameVersion === null) {
    return loaderLabel;
  }
  return `${gameVersion} · ${loaderLabel}`;
}
