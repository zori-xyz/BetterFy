import type { ReactNode } from "react";

export default function AccentTitle({ text, trailing }: { text: string; trailing?: ReactNode }) {
  // Regular spaces separate semantic words; a non-breaking space deliberately
  // keeps names such as “Dota 2” together inside the gradient tail.
  const parts = text.trim().split(/ +/);
  const accent = parts.pop() ?? text;

  return (
    <>
      {parts.length > 0 && <>{parts.join(" ")}{" "}</>}
      <span className="accent-tail"><em>{accent}</em>{trailing}</span>
    </>
  );
}
