import type { DownloadProgress } from "@/lib/generated/DownloadProgress";

function ratio(done: number, total: number): number {
  if (total === 0) {
    return 0;
  }
  return Math.min(100, (done / total) * 100);
}

export function barValue(progress: DownloadProgress): number {
  if (progress.bytesTotal === 0) {
    return ratio(progress.finished, progress.total);
  }
  return ratio(progress.bytesDone, progress.bytesTotal);
}

function oneDecimal(whole: number, fraction: number): string {
  return `${Math.floor(whole)}.${fraction}`;
}

function megabytes(bytes: number): string {
  const tenths = Math.floor((bytes * 10) / 1_000_000);
  return oneDecimal(tenths / 10, tenths % 10);
}

export function progressText(progress: DownloadProgress): string {
  const tenths =
    progress.bytesTotal === 0
      ? 0
      : Math.floor((progress.bytesDone * 1000) / progress.bytesTotal);
  return `${oneDecimal(tenths / 10, tenths % 10)}%  ${megabytes(progress.bytesDone)}MB/${megabytes(progress.bytesTotal)}MB`;
}
