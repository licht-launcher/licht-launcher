export type InstanceVersionInfo = {
  gameVersion: string | null;
  loaderLabel: string;
};

/**
 * Best-effort parse of Licht / Mojang version ids for display.
 * Fabric/Quilt: `fabric-loader-<loader>-<game>`. NeoForge installations use
 * `neoforge-<game>-<build>`; legacy Forge-like ids remain supported.
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
  const neoforge = /^neoforge-(\d+\.\d+(?:\.\d+)?)-(.+)$/.exec(versionId);
  if (neoforge !== null) {
    return {
      gameVersion: neoforge[1],
      loaderLabel: `NeoForge ${neoforge[2]}`,
    };
  }
  const legacyNeoforge = /^neoforge-(.+)$/.exec(versionId);
  if (legacyNeoforge !== null) {
    const build = /^(\d+)\.(\d+)(?:\.\d+)?$/.exec(legacyNeoforge[1]);
    const gameVersion = build === null ? null : `1.${build[1]}.${build[2]}`;
    return {
      gameVersion,
      loaderLabel: `NeoForge ${legacyNeoforge[1]}`,
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
