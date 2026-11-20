// Sparkline geometry: pure, so it is tested without a DOM.

/**
 * A sparkline's polyline points in a `width` × `height` box with 1 px of room for the stroke.
 * A flat or empty series lies on the baseline.
 */
export function sparkPoints(values: readonly number[], width: number, height: number): string {
  if (values.length === 0) return "";
  const max = Math.max(...values);
  const step = values.length > 1 ? (width - 2) / (values.length - 1) : 0;
  return values
    .map((v, i) => {
      const x = 1 + i * step;
      const y = height - 1 - (max > 0 ? (v / max) * (height - 2) : 0);
      return `${round(x)},${round(y)}`;
    })
    .join(" ");
}

const round = (n: number) => Math.round(n * 10) / 10;
