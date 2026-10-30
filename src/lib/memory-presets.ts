/** Sparse reference marks; allocation itself is independent of these values. */
export function memoryPresetOptions(input: {
  minimumMb: number;
  maximumMb: number;
  recommendedMb: number;
  totalMb: number;
}): number[] {
  const ceiling = Math.max(
    input.minimumMb,
    Math.min(input.maximumMb, input.totalMb),
  );
  const range = ceiling - input.minimumMb;
  const options = new Set<number>([input.minimumMb, ceiling]);
  for (const fraction of [0.25, 0.5, 0.75]) {
    const mark = Math.round((input.minimumMb + range * fraction) / 1024) * 1024;
    if (mark > input.minimumMb && mark < ceiling) options.add(mark);
  }
  if (
    input.recommendedMb > input.minimumMb &&
    input.recommendedMb < ceiling &&
    [...options].every(
      (mark) => Math.abs(mark - input.recommendedMb) >= range / 8,
    )
  ) {
    options.add(input.recommendedMb);
  }
  return [...options].sort((left, right) => left - right);
}

export function formatMemoryPreset(mb: number): string {
  if (mb >= 1024) {
    return `${Number((mb / 1024).toFixed(1))}GB`;
  }
  return `${mb}MB`;
}
