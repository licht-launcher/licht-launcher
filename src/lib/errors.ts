const FRIENDLY: Record<string, string> = {
  "HTTP request failed":
    "The network request failed. Check your connection and try again.",
  "I/O operation failed":
    "The disk could not be written. Free some space and try again.",
  "downloaded SHA1 does not match": "A downloaded file was damaged. Try again.",
  "downloaded SHA512 does not match":
    "A downloaded file was damaged. Try again.",
  "downloaded MD5 does not match": "A downloaded file was damaged. Try again.",
  "LICHT_MSA_CLIENT_ID is missing or empty":
    "Microsoft sign-in is not configured. Set LICHT_MSA_CLIENT_ID and try again.",
  "Microsoft sign-in timed out":
    "Microsoft sign-in timed out. Start sign-in again.",
  "could not open the system browser for Microsoft sign-in":
    "Could not open the browser for Microsoft sign-in. Try again.",
  "Microsoft sign-in state mismatch":
    "Microsoft sign-in failed a security check. Start sign-in again.",
  "Microsoft sign-in was declined": "Microsoft sign-in was cancelled.",
  "Minecraft services rejected this app (not allow-listed yet)":
    "Microsoft sign-in is waiting on Mojang approval for this app. Try again later.",
  "this Microsoft account does not own Minecraft: Java Edition":
    "This Microsoft account does not own Minecraft: Java Edition.",
  "this Microsoft account is a child and needs to join a Family":
    "This Microsoft account is a child account and must join a Family first.",
  "this Microsoft account has no Xbox profile":
    "This Microsoft account needs an Xbox profile before signing in.",
  "Minecraft profile was not found for this account":
    "No Minecraft profile was found. Open the official launcher once, then try again.",
  "no signed-in Microsoft account": "No Microsoft account is signed in.",
  "secure credential storage failed":
    "Could not store credentials in the OS keyring.",
  "mod loader version was not found":
    "No loader build matches this game version. Try another version or channel.",
  "version was not found":
    "That game version is missing. Install it again or pick another version.",
  "version manifest is unavailable":
    "Could not load the version list. Check your connection and try again.",
  "LICHT_CURSEFORGE_API_KEY is missing or empty":
    "CurseForge needs LICHT_CURSEFORGE_API_KEY. Modrinth packs still work.",
  "curseforge download host is not allowed":
    "A CurseForge file URL is not on an allowed download host.",
  "curseforge pack disallows third-party distribution":
    "This CurseForge pack cannot be downloaded because its author " +
    "disabled third-party distribution.",
  "modpack archive is invalid":
    "That modpack archive could not be read. Try another version.",
  "unknown platform": "That platform is not supported.",
  "unknown channel": "That loader channel is not supported.",
  "unknown modpack source": "That modpack source is not supported.",
};

export function messageOf(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return "Something went wrong.";
}

const DISTRIBUTION_DENIED =
  "curseforge mods disallow third-party distribution:";
const FILE_MISSING = "curseforge file was not found:";

export function friendlyError(message: string): string {
  if (message.startsWith(DISTRIBUTION_DENIED)) {
    const mods = message.slice(DISTRIBUTION_DENIED.length).trim();
    if (mods.length === 0) {
      return (
        "Some mods cannot be downloaded because their authors disabled " +
        "third-party distribution. The pack was not installed."
      );
    }
    return (
      "These mods cannot be downloaded because their authors disabled " +
      `third-party distribution: ${mods}. The pack was not installed.`
    );
  }
  if (message.startsWith(FILE_MISSING)) {
    const fileId = message.slice(FILE_MISSING.length).trim();
    return (
      `A file in this pack is no longer on CurseForge (${fileId}). ` +
      "The pack was not installed."
    );
  }
  return FRIENDLY[message] ?? message;
}
