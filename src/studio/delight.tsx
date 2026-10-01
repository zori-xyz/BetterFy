import { useEffect, useRef, useState, type MouseEvent } from "react";
import { flushSync } from "react-dom";

function motionAllowed() {
  return (
    !document.documentElement.classList.contains("motion-disabled") &&
    !window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

type ViewTransitionDocument = Document & {
  startViewTransition?: (update: () => void) => { ready: Promise<void> };
};

// Switches the theme with a circle that grows from the clicked control.
// Uses the View Transitions API where WebView2/Chromium provides it and
// falls back to an instant switch.
export function revealTheme(event: MouseEvent, apply: () => void) {
  const doc = document as ViewTransitionDocument;
  if (!doc.startViewTransition || !motionAllowed()) {
    apply();
    return;
  }
  const x = event.clientX;
  const y = event.clientY;
  const radius = Math.hypot(
    Math.max(x, window.innerWidth - x),
    Math.max(y, window.innerHeight - y),
  );
  const transition = doc.startViewTransition(() => flushSync(apply));
  transition.ready
    .then(() => {
      document.documentElement.animate(
        {
          clipPath: [`circle(0px at ${x}px ${y}px)`, `circle(${radius}px at ${x}px ${y}px)`],
        },
        {
          duration: 650,
          easing: "cubic-bezier(.23,1,.32,1)",
          pseudoElement: "::view-transition-new(root)",
        },
      );
    })
    .catch(() => undefined);
}

// Counts up to a number the first time it is shown.
export function CountUp({ value, locale }: { value: number; locale: string }) {
  const [shown, setShown] = useState(() => (motionAllowed() ? 0 : value));
  const from = useRef(0);
  useEffect(() => {
    if (!motionAllowed()) {
      setShown(value);
      return;
    }
    const start = performance.now();
    const origin = from.current;
    let frame = 0;
    const step = (time: number) => {
      const progress = Math.min(1, (time - start) / 1100);
      const eased = 1 - (1 - progress) ** 4;
      setShown(Math.round(origin + (value - origin) * eased));
      if (progress < 1) frame = requestAnimationFrame(step);
      else from.current = value;
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  }, [value]);
  return <>{shown.toLocaleString(locale)}</>;
}
