interface Props {
  /** Oldest first; missing values are drawn as 0. */
  values: (number | null)[];
  max: number;
  /** A CSS color, usually a theme variable. */
  color: string;
}

const W = 240;
const H = 56;

/** A small area graph of the last minute. */
export function Sparkline({ values, max, color }: Props) {
  if (values.length < 2 || max <= 0) return <svg className="sparkline" viewBox={`0 0 ${W} ${H}`} aria-hidden="true" />;
  const step = W / (values.length - 1);
  const points = values.map((v, i) => {
    const y = H - (Math.min(v ?? 0, max) / max) * (H - 2) - 1;
    return `${(i * step).toFixed(1)},${y.toFixed(1)}`;
  });
  const line = points.join(" ");
  return (
    <svg className="sparkline" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" aria-hidden="true">
      <polygon points={`0,${H} ${line} ${W},${H}`} style={{ fill: color, opacity: 0.16 }} />
      <polyline points={line} style={{ fill: "none", stroke: color }} strokeWidth={1.6} vectorEffect="non-scaling-stroke" />
    </svg>
  );
}
