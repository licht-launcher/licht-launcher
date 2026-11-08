import { Gauge } from "lucide-react";

import { speedChartStyles } from "./speed-chart.styles";

export type SpeedSample = { at: number; bytesPerSecond: number };
export const SPEED_HISTORY_MS = 30_000;
export const SPEED_SAMPLE_MS = 1000;
const BUCKETS = SPEED_HISTORY_MS / SPEED_SAMPLE_MS;

export function formatSpeed(bytesPerSecond: number): string {
  if (bytesPerSecond >= 1_000_000) {
    return `${(bytesPerSecond / 1_000_000).toFixed(1)} MB/s`;
  }
  if (bytesPerSecond >= 1000) {
    return `${(bytesPerSecond / 1000).toFixed(1)} KB/s`;
  }
  return `${Math.round(bytesPerSecond)} B/s`;
}

export function SpeedChart({
  title,
  series,
}: {
  title: string;
  series: readonly {
    label: string;
    samples: readonly SpeedSample[];
    color: "violet" | "cool";
    kind?: "bars" | "line";
  }[];
}) {
  const colors = speedChartStyles.colors;
  const end = Math.max(
    0,
    ...series.map((line) => line.samples.at(-1)?.at ?? 0),
  );
  const visible = series.map((line) => ({
    ...line,
    samples: line.samples.filter(
      (sample) => sample.at > end - SPEED_HISTORY_MS,
    ),
  }));
  // Each sample is already a closed second; rendering must not recalculate it.
  const maximum = Math.max(
    1000,
    ...visible.flatMap((line) =>
      line.samples.map((sample) => sample.bytesPerSecond),
    ),
  );
  const summary = visible
    .map(
      (line) =>
        `${line.label}: ${formatSpeed(line.samples.at(-1)?.bytesPerSecond ?? 0)}`,
    )
    .join("; ");
  function points(samples: readonly SpeedSample[]) {
    return samples
      .map((sample) => {
        const x =
          ((sample.at - end + SPEED_HISTORY_MS) / SPEED_HISTORY_MS) * 1000 -
          1000 / BUCKETS / 2;
        const y = 150 - (Math.max(0, sample.bytesPerSecond) / maximum) * 140;
        return `${x.toFixed(2)},${y.toFixed(2)}`;
      })
      .join(" ");
  }

  function bars(samples: readonly SpeedSample[]) {
    const width = (1000 / BUCKETS) * 0.4;
    return samples.map((sample) => {
      const height = (sample.bytesPerSecond / maximum) * 140;
      return (
        <rect
          key={sample.at}
          x={
            ((sample.at - end + SPEED_HISTORY_MS) / SPEED_HISTORY_MS) * 1000 -
            1000 / BUCKETS / 2 -
            width / 2
          }
          y={150 - height}
          width={width}
          height={height}
        />
      );
    });
  }

  return (
    <section aria-label={title} className={speedChartStyles.root}>
      <h2 className={speedChartStyles.title}>
        <Gauge aria-hidden className={speedChartStyles.titleIcon} />
        {title}
      </h2>
      <ul aria-label="Speed legend" className={speedChartStyles.legend}>
        {visible.map((line) => (
          <li key={line.label} className={speedChartStyles.legendItem}>
            <svg
              aria-hidden
              viewBox="0 0 16 16"
              className={`${speedChartStyles.legendSwatch} ${colors[line.color]}`}
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
            >
              <path
                d={
                  line.kind === "bars" ? "M3 14V9 M8 14V3 M13 14V6" : "M1 8H15"
                }
              />
            </svg>
            <span className={speedChartStyles.legendLabel}>{line.label}</span>
            <span
              className={`${speedChartStyles.legendValue} ${line.kind === "bars" ? speedChartStyles.legendValueBars : speedChartStyles.legendValueLine}`}
            >
              {formatSpeed(line.samples.at(-1)?.bytesPerSecond ?? 0)}
            </span>
          </li>
        ))}
      </ul>
      <svg
        role="img"
        aria-label={`${title}. ${summary}. Last ${SPEED_HISTORY_MS / 1000} seconds.`}
        viewBox="0 0 1000 160"
        preserveAspectRatio="none"
        className={speedChartStyles.chart}
      >
        <title>
          {title} over the last {SPEED_HISTORY_MS / 1000} seconds
        </title>
        <path
          d="M0 10H1000 M0 80H1000 M0 150H1000"
          fill="none"
          stroke="currentColor"
          className={speedChartStyles.grid}
          vectorEffect="non-scaling-stroke"
        />
        {[...visible]
          .sort(
            (left, right) =>
              Number(left.kind === "bars") - Number(right.kind === "bars"),
          )
          .map(
            (line) =>
              line.samples.length > 0 &&
              (line.kind === "bars" ? (
                <g
                  key={line.label}
                  aria-label={`${line.label} bars`}
                  className={colors[line.color]}
                  fill="currentColor"
                >
                  {bars(line.samples)}
                </g>
              ) : (
                <polyline
                  key={line.label}
                  aria-label={`${line.label} line`}
                  className={`${colors[line.color]} ${speedChartStyles.line}`}
                  points={points(line.samples)}
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  vectorEffect="non-scaling-stroke"
                />
              )),
          )}
      </svg>
    </section>
  );
}
