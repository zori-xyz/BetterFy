import { useId } from "react";

/**
 * Scalable adaptation of the founder-provided three-capsule BetterFy mark.
 * The source image defines material and silhouette; this SVG is the UI master.
 */
export default function BetterFyMark({
  className = "",
  title,
}: {
  className?: string;
  title?: string;
}) {
  const uid = useId().replace(/:/g, "");
  const body = `bf-capsule-body-${uid}`;
  const edge = `bf-capsule-edge-${uid}`;
  const light = `bf-capsule-light-${uid}`;
  const glow = `bf-capsule-glow-${uid}`;

  return (
    <svg
      className={`betterfy-mark ${className}`}
      viewBox="0 0 150 112"
      role={title ? "img" : undefined}
      aria-label={title}
      aria-hidden={title ? undefined : true}
    >
      <defs>
        <linearGradient id={body} x1=".08" y1=".02" x2=".9" y2=".98">
          <stop offset="0" stopColor="#6b18ac" />
          <stop offset=".26" stopColor="#32114f" />
          <stop offset=".58" stopColor="#4d1377" />
          <stop offset=".82" stopColor="#8f20d7" />
          <stop offset="1" stopColor="#270d3d" />
        </linearGradient>
        <linearGradient id={edge} x1="0" y1=".1" x2="1" y2=".9">
          <stop stopColor="#f2c3ff" />
          <stop offset=".18" stopColor="#b640ff" />
          <stop offset=".5" stopColor="#6e18bd" />
          <stop offset=".78" stopColor="#e768ff" />
          <stop offset="1" stopColor="#7b20c5" />
        </linearGradient>
        <linearGradient id={light} x1=".08" y1=".12" x2=".95" y2=".9">
          <stop stopColor="#ffffff" stopOpacity=".9" />
          <stop offset=".2" stopColor="#e896ff" stopOpacity=".74" />
          <stop offset=".52" stopColor="#a62aff" stopOpacity=".06" />
          <stop offset=".82" stopColor="#ff9aff" stopOpacity=".74" />
          <stop offset="1" stopColor="#ffffff" stopOpacity=".18" />
        </linearGradient>
        <filter id={glow} x="-45%" y="-35%" width="190%" height="180%">
          <feDropShadow dx="0" dy="0" stdDeviation="4" floodColor="#ba36ff" floodOpacity=".48" />
          <feDropShadow dx="0" dy="5" stdDeviation="5" floodColor="#160322" floodOpacity=".68" />
        </filter>
      </defs>

      {[0, 48, 96].map((x, index) => (
        <g
          className={`mark-capsule mark-capsule-${index + 1}`}
          filter={`url(#${glow})`}
          transform={`translate(${x} 0)`}
          key={x}
        >
          <path
            d="M20 6h16c7 0 11 6 9 12L27 96c-1 6-6 10-12 10H11c-7 0-11-6-9-12L16 16C17 10 20 6 20 6Z"
            fill={`url(#${body})`}
            stroke={`url(#${edge})`}
            strokeWidth="2.2"
            strokeLinejoin="round"
          />
          <path
            d="M23 11h11c4 0 6 3 5 7L22 91c-.7 4-3 7-7 8"
            fill="none"
            stroke={`url(#${light})`}
            strokeWidth="2"
            strokeLinecap="round"
            opacity=".92"
          />
          <path
            d="M10 67c9-5 18-6 26-2"
            fill="none"
            stroke="#ec75ff"
            strokeWidth="5"
            strokeLinecap="round"
            opacity=".24"
          />
        </g>
      ))}
    </svg>
  );
}
