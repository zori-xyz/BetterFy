// Pointer light for elements marked data-spotlight: sets --spot-x/--spot-y
// (pixels inside the element) so CSS can draw a glow where the pointer is.
// data-spotlight="tilt" also sets --tilt-x/--tilt-y for a slight 3D lean.
// One delegated listener serves every marked element.

let current: HTMLElement | null = null;

function clear(element: HTMLElement) {
  element.style.removeProperty("--spot-x");
  element.style.removeProperty("--spot-y");
  element.style.removeProperty("--tilt-x");
  element.style.removeProperty("--tilt-y");
  element.removeAttribute("data-spot-active");
}

function motionAllowed() {
  return (
    !document.documentElement.classList.contains("motion-disabled") &&
    !window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

if (typeof window !== "undefined") {
  window.addEventListener(
    "pointermove",
    (event) => {
      if (event.pointerType === "touch") return;
      const target = (event.target as Element | null)?.closest<HTMLElement>("[data-spotlight]");
      if (current && current !== target) {
        clear(current);
        current = null;
      }
      if (!target || !motionAllowed()) return;
      current = target;
      const bounds = target.getBoundingClientRect();
      const x = event.clientX - bounds.left;
      const y = event.clientY - bounds.top;
      target.style.setProperty("--spot-x", `${x}px`);
      target.style.setProperty("--spot-y", `${y}px`);
      target.setAttribute("data-spot-active", "");
      if (target.dataset.spotlight === "tilt") {
        target.style.setProperty("--tilt-x", `${((y / bounds.height) * 2 - 1) * -2.2}deg`);
        target.style.setProperty("--tilt-y", `${((x / bounds.width) * 2 - 1) * 2.8}deg`);
      }
    },
    { passive: true },
  );
  document.addEventListener("pointerleave", () => {
    if (current) clear(current);
    current = null;
  });
}
