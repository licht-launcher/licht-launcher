export const DOWNLOAD_SPEED_ERROR =
  "Download speed limit must be a positive number of kB/s with at most three decimal places.";

export function parseDownloadLimit(value: string): number | null | "invalid" {
  const text = value.trim();
  if (text === "") return null;
  if (!/^(?:\d+(?:\.\d{0,3})?|\.\d{1,3})$/.test(text)) return "invalid";
  const [whole, fraction = ""] = text.split(".");
  const bytes = Number(whole) * 1000 + Number(fraction.padEnd(3, "0"));
  return Number.isSafeInteger(bytes) && bytes > 0 ? bytes : "invalid";
}

export function downloadLimitKbs(bytes: number | null): string {
  if (bytes === null) return "";
  const whole = Math.floor(bytes / 1000);
  const fraction = bytes % 1000;
  return fraction === 0
    ? String(whole)
    : `${whole}.${String(fraction).padStart(3, "0").replace(/0+$/, "")}`;
}

export function formatDownloadLimit(bytes: number): string {
  const mib = bytes / (1024 * 1024);
  if (mib < 0.01) return "< 0.01 MiB/s";
  return `${new Intl.NumberFormat("en-US", { maximumFractionDigits: 2 }).format(mib)} MiB/s`;
}
