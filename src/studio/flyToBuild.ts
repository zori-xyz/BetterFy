// Sends a small copy of a mod's artwork from the place the person clicked to
// the "My build" item in the sidebar, so adding a mod has a visible
// destination. Keyboard activation has no pointer position and skips the
// flight; the counter bump in the sidebar still marks the change.

let lastPointer: { x: number; y: number; at: number } | null = null;

if (typeof window !== "undefined") {
  window.addEventListener(
    "pointerdown",
    (event) => {
      lastPointer = { x: event.clientX, y: event.clientY, at: performance.now() };
    },
    { capture: true, passive: true },
  );
}

function motionAllowed() {
  return (
    !document.documentElement.classList.contains("motion-disabled") &&
    !window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

export function flyToBuild(image: string | null | undefined) {
  if (!motionAllowed() || !lastPointer || performance.now() - lastPointer.at > 1000) return;
  const target = document.querySelector<HTMLElement>(".s-nav-main [data-nav=build]");
  if (!target) return;
  const end = target.getBoundingClientRect();
  if (!end.width) return;
  const size = 38;
  const start = { x: lastPointer.x - size / 2, y: lastPointer.y - size / 2 };
  const dx = end.left + 20 - size / 2 - start.x;
  const dy = end.top + end.height / 2 - size / 2 - start.y;
  const chip = document.createElement("div");
  chip.className = "s-fly-to-build";
  chip.setAttribute("aria-hidden", "true");
  if (image) chip.style.backgroundImage = `url("${image.replace(/"/g, "%22")}")`;
  chip.style.left = `${start.x}px`;
  chip.style.top = `${start.y}px`;
  document.body.appendChild(chip);
  const lift = Math.min(90, Math.abs(dy) / 2 + 40);
  const flight = chip.animate(
    [
      { transform: "translate(0, 0) scale(.6)", opacity: 0 },
      { transform: "translate(0, -10px) scale(1)", opacity: 1, offset: 0.12 },
      {
        transform: `translate(${dx * 0.55}px, ${Math.min(dy, 0) * 0.55 - lift}px) scale(.85)`,
        opacity: 1,
        offset: 0.55,
      },
      { transform: `translate(${dx}px, ${dy}px) scale(.3)`, opacity: 0.4 },
    ],
    { duration: 720, easing: "cubic-bezier(.45,0,.25,1)" },
  );
  const done = () => chip.remove();
  flight.onfinish = done;
  flight.oncancel = done;
}
