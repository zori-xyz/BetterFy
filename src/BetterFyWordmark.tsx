export default function BetterFyWordmark({
  compact = false,
  animated = false,
  hero = false,
}: {
  compact?: boolean;
  animated?: boolean;
  hero?: boolean;
}) {
  return (
    <div
      className={`wordmark${compact ? " is-compact" : ""}${animated ? " is-animated" : ""}${hero ? " is-hero" : ""}`}
      aria-label="BetterFy"
    >
      <span className="wordmark-better">Better</span>
      <span className="wordmark-fy">Fy</span>
    </div>
  );
}
