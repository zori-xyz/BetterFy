import { useEffect, useRef } from "react";

type Speck = {
  x: number;
  y: number;
  size: number;
  depth: number;
  phase: number;
  pushX: number;
  pushY: number;
};
type Ripple = { x: number; y: number; start: number };
type Comet = { x: number; y: number; start: number; length: number };

// Background of the sign-in screens: slow violet light, drifting specks that
// make room for the pointer and link up into a small constellation around
// it, an occasional comet, and a ripple where the person clicks empty space.
// Runs at most ~30 fps and stops entirely when motion is off or the window
// is hidden.
const REACH = 150;
const LINK = 112;

export default function AuthAmbient() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    const host = canvas?.parentElement;
    const context = canvas?.getContext("2d", { alpha: true });
    if (!canvas || !host || !context) return;

    const motion = window.matchMedia("(prefers-reduced-motion: reduce)");
    const specks: Speck[] = Array.from({ length: 96 }, (_, index) => ({
      x: ((index * 73.37 + 17) % 100) / 100,
      y: ((index * 39.83 + 43) % 100) / 100,
      size: index % 11 === 0 ? 1.6 : index % 3 === 0 ? 1.05 : 0.65,
      depth: 0.35 + (index % 7) / 9,
      phase: index * 1.41,
      pushX: 0,
      pushY: 0,
    }));
    const ripples: Ripple[] = [];
    let comet: Comet | null = null;
    let nextComet = 4000;
    let width = 0;
    let height = 0;
    let frame = 0;
    let lastFrame = 0;
    let pointerX = -1;
    let pointerY = -1;
    let glowX = 0;
    let glowY = 0;
    let active = false;

    const shouldAnimate = () =>
      !motion.matches &&
      !document.documentElement.classList.contains("motion-disabled") &&
      !document.hidden;

    const paint = (time: number) => {
      context.clearRect(0, 0, width, height);
      const moving = shouldAnimate();
      const seconds = moving ? time / 1000 : 0;
      const light = document.documentElement.dataset.theme === "light";
      const ink = light ? "78, 38, 103" : "217, 187, 239";
      const violet = light ? "120, 53, 169" : "196, 120, 239";

      // Two slow pools of light, plus a softer one that trails the pointer.
      const pools = [
        {
          x: width * (0.16 + Math.sin(seconds * 0.07) * 0.06),
          y: height * (0.78 + Math.cos(seconds * 0.05) * 0.05),
          r: Math.max(width, height) * 0.5,
          color: violet,
          alpha: light ? 0.08 : 0.11,
        },
        {
          x: width * (0.86 + Math.cos(seconds * 0.06) * 0.05),
          y: height * (0.12 + Math.sin(seconds * 0.08) * 0.06),
          r: Math.max(width, height) * 0.42,
          color: light ? "172, 77, 140" : "207, 75, 201",
          alpha: light ? 0.05 : 0.07,
        },
      ];
      if (pointerX >= 0) {
        glowX += (pointerX - glowX) * 0.08;
        glowY += (pointerY - glowY) * 0.08;
        pools.push({ x: glowX, y: glowY, r: 260, color: violet, alpha: light ? 0.06 : 0.08 });
      }
      for (const pool of pools) {
        const gradient = context.createRadialGradient(pool.x, pool.y, 0, pool.x, pool.y, pool.r);
        gradient.addColorStop(0, `rgba(${pool.color}, ${pool.alpha})`);
        gradient.addColorStop(1, `rgba(${pool.color}, 0)`);
        context.fillStyle = gradient;
        context.fillRect(0, 0, width, height);
      }

      const points: { x: number; y: number; near: number }[] = [];
      for (const speck of specks) {
        speck.pushX *= 0.92;
        speck.pushY *= 0.92;
        let x = speck.x * width + speck.pushX;
        let y =
          ((speck.y * height + seconds * (2.5 + speck.depth * 4)) % (height + 20)) -
          10 +
          speck.pushY;
        let near = 0;
        if (pointerX >= 0) {
          const dx = x - pointerX;
          const dy = y - pointerY;
          const distance = Math.hypot(dx, dy);
          if (distance < REACH && distance > 0.1) {
            near = 1 - distance / REACH;
            const shift = near * near * 22 * speck.depth;
            x += (dx / distance) * shift;
            y += (dy / distance) * shift;
          }
        }
        points.push({ x, y, near });
        const alpha =
          (light ? 0.16 : 0.19) + (Math.sin(seconds * 0.7 + speck.phase) + 1) * 0.055 + near * 0.45;
        context.fillStyle = `rgba(${near > 0 ? violet : ink}, ${Math.min(alpha, 0.85)})`;
        context.beginPath();
        context.arc(x, y, speck.size + near * 0.9, 0, Math.PI * 2);
        context.fill();
      }

      // Constellation: link specks that are close to the pointer and to each other.
      context.lineWidth = 0.7;
      for (let a = 0; a < points.length; a += 1) {
        if (!points[a].near) continue;
        for (let b = a + 1; b < points.length; b += 1) {
          if (!points[b].near) continue;
          const distance = Math.hypot(points[a].x - points[b].x, points[a].y - points[b].y);
          if (distance > LINK) continue;
          const alpha = (1 - distance / LINK) * Math.min(points[a].near, points[b].near) * 0.9;
          context.strokeStyle = `rgba(${violet}, ${alpha})`;
          context.beginPath();
          context.moveTo(points[a].x, points[a].y);
          context.lineTo(points[b].x, points[b].y);
          context.stroke();
        }
      }

      if (!moving) return;

      for (let index = ripples.length - 1; index >= 0; index -= 1) {
        const ripple = ripples[index];
        const progress = (time - ripple.start) / 900;
        if (progress >= 1) {
          ripples.splice(index, 1);
          continue;
        }
        const eased = 1 - (1 - progress) ** 3;
        context.strokeStyle = `rgba(${violet}, ${(1 - progress) * 0.55})`;
        context.lineWidth = 1.4;
        context.beginPath();
        context.arc(ripple.x, ripple.y, 8 + eased * 130, 0, Math.PI * 2);
        context.stroke();
      }

      if (!comet && time > nextComet) {
        comet = {
          x: width * (0.1 + ((time / 977) % 1) * 0.5),
          y: height * (0.05 + ((time / 1531) % 1) * 0.2),
          start: time,
          length: 90 + ((time / 311) % 1) * 70,
        };
      }
      if (comet) {
        const progress = (time - comet.start) / 1100;
        if (progress >= 1) {
          comet = null;
          nextComet = time + 6500 + ((time / 113) % 1) * 5000;
        } else {
          const travel = progress * Math.min(width, 900) * 0.55;
          const headX = comet.x + travel;
          const headY = comet.y + travel * 0.42;
          const tailX = headX - comet.length;
          const tailY = headY - comet.length * 0.42;
          const fade = Math.sin(progress * Math.PI);
          const tail = context.createLinearGradient(tailX, tailY, headX, headY);
          tail.addColorStop(0, `rgba(${ink}, 0)`);
          tail.addColorStop(1, `rgba(${ink}, ${0.75 * fade})`);
          context.strokeStyle = tail;
          context.lineWidth = 1.3;
          context.beginPath();
          context.moveTo(tailX, tailY);
          context.lineTo(headX, headY);
          context.stroke();
        }
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
    const local = (event: PointerEvent) => {
      const bounds = host.getBoundingClientRect();
      return { x: event.clientX - bounds.left, y: event.clientY - bounds.top };
    };
    const onPointer = (event: PointerEvent) => {
      if (event.pointerType === "touch") return;
      const point = local(event);
      if (pointerX < 0) {
        glowX = point.x;
        glowY = point.y;
      }
      pointerX = point.x;
      pointerY = point.y;
    };
    const onLeave = () => {
      pointerX = -1;
      pointerY = -1;
    };
    const onDown = (event: PointerEvent) => {
      if (!active) return;
      const target = event.target as Element | null;
      if (target?.closest("button, a, input, label, select, textarea, .auth-login-card")) return;
      const point = local(event);
      ripples.push({ x: point.x, y: point.y, start: performance.now() });
      for (const speck of specks) {
        const x = speck.x * width + speck.pushX;
        const dx = x - point.x;
        const y = speck.y * height;
        const dy = y - point.y;
        const distance = Math.hypot(dx, dy) || 1;
        if (distance < 220) {
          const force = (1 - distance / 220) * 34;
          speck.pushX += (dx / distance) * force;
          speck.pushY += (dy / distance) * force;
        }
      }
    };
    const observer = new ResizeObserver(resize);
    const classObserver = new MutationObserver(sync);
    observer.observe(host);
    classObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class", "data-theme"],
    });
    window.addEventListener("pointermove", onPointer, { passive: true });
    document.addEventListener("pointerleave", onLeave);
    host.addEventListener("pointerdown", onDown);
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
      document.removeEventListener("pointerleave", onLeave);
      host.removeEventListener("pointerdown", onDown);
      document.removeEventListener("visibilitychange", sync);
      motion.removeEventListener("change", sync);
    };
  }, []);

  return <canvas className="auth-id-ambient" ref={canvasRef} aria-hidden="true" />;
}
