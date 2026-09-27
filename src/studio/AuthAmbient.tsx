import { useEffect, useRef } from "react";

type Speck = { x: number; y: number; size: number; depth: number; phase: number };

export default function AuthAmbient() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    const host = canvas?.parentElement;
    const context = canvas?.getContext("2d", { alpha: true });
    if (!canvas || !host || !context) return;

    const motion = window.matchMedia("(prefers-reduced-motion: reduce)");
    const specks: Speck[] = Array.from({ length: 54 }, (_, index) => ({
      x: ((index * 73.37 + 17) % 100) / 100,
      y: ((index * 39.83 + 43) % 100) / 100,
      size: index % 11 === 0 ? 1.55 : index % 3 === 0 ? 1.05 : 0.65,
      depth: 0.35 + (index % 7) / 9,
      phase: index * 1.41,
    }));
    let width = 0;
    let height = 0;
    let frame = 0;
    let lastFrame = 0;
    let driftX = 0;
    let driftY = 0;
    let pointerX = 0;
    let pointerY = 0;
    let active = false;

    const shouldAnimate = () =>
      !motion.matches &&
      !document.documentElement.classList.contains("motion-disabled") &&
      !document.hidden;
    const paint = (time: number) => {
      context.clearRect(0, 0, width, height);
      const moving = shouldAnimate();
      if (moving) {
        driftX += (pointerX - driftX) * 0.035;
        driftY += (pointerY - driftY) * 0.035;
      }
      const seconds = moving ? time / 1000 : 0;
      const light = document.documentElement.dataset.theme === "light";
      for (const speck of specks) {
        const x = speck.x * width + driftX * speck.depth * 19;
        const y =
          ((speck.y * height + seconds * (2.5 + speck.depth * 4)) % (height + 20)) -
          10 +
          driftY * speck.depth * 13;
        const alpha = (light ? 0.16 : 0.19) + (Math.sin(seconds * 0.7 + speck.phase) + 1) * 0.055;
        context.fillStyle = light ? `rgba(78, 38, 103, ${alpha})` : `rgba(217, 187, 239, ${alpha})`;
        context.beginPath();
        context.arc(x, y, speck.size, 0, Math.PI * 2);
        context.fill();
      }
    };
    const tick = (time: number) => {
      if (!active) return;
      if (time - lastFrame >= 32) {
        paint(time);
        lastFrame = time;
      }
      frame = window.requestAnimationFrame(tick);
    };
    const sync = () => {
      const animate = shouldAnimate();
      if (animate !== active) {
        active = animate;
        if (active) frame = window.requestAnimationFrame(tick);
        else window.cancelAnimationFrame(frame);
      }
      if (!animate) paint(0);
    };
    const resize = () => {
      const bounds = host.getBoundingClientRect();
      width = bounds.width;
      height = bounds.height;
      const ratio = Math.min(window.devicePixelRatio || 1, 2);
      canvas.width = Math.max(1, Math.round(width * ratio));
      canvas.height = Math.max(1, Math.round(height * ratio));
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      paint(0);
    };
    const onPointer = (event: PointerEvent) => {
      if (event.pointerType === "touch") return;
      pointerX = (event.clientX / Math.max(window.innerWidth, 1)) * 2 - 1;
      pointerY = (event.clientY / Math.max(window.innerHeight, 1)) * 2 - 1;
    };
    const observer = new ResizeObserver(resize);
    const classObserver = new MutationObserver(sync);
    observer.observe(host);
    classObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class", "data-theme"],
    });
    window.addEventListener("pointermove", onPointer, { passive: true });
    document.addEventListener("visibilitychange", sync);
    motion.addEventListener("change", sync);
    resize();
    sync();
    return () => {
      active = false;
      window.cancelAnimationFrame(frame);
      observer.disconnect();
      classObserver.disconnect();
      window.removeEventListener("pointermove", onPointer);
      document.removeEventListener("visibilitychange", sync);
      motion.removeEventListener("change", sync);
    };
  }, []);

  return <canvas className="auth-id-ambient" ref={canvasRef} aria-hidden="true" />;
}
