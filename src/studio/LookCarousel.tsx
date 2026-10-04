import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { ArrowUpRight, ChevronLeft, ChevronRight } from "lucide-react";
import { useLocale } from "../i18n";
import { featuredMods, mods, type StudioMod } from "./model";
import { Media } from "./ui";
import { useMotion } from "./useMotion";
import "./studio-shelf.css";

// A showcase shelf: one look in focus, its neighbours turned away in depth.
// It advances on its own every few seconds and stops while the player is
// looking (hover, focus, drag) or the shelf is off screen. Dragging snaps to
// the nearest look with a little momentum; side looks come to the front on
// click; the look in front opens. The stage light takes the colour of the
// look in front.
const AUTO_ADVANCE = 4600;
const VISIBLE_SIDE = 2;
const displayIds = [
  ...featuredMods.map((mod) => mod.id),
  "couriers-baby-roshan-crownfall",
  "trees-crystals-trees",
  "roshan-winter-roshan",
  "huds-blue-web-hud",
  "river-blue-river",
  "wards-curious-snaptrap",
];
const looks = displayIds.flatMap((id) => mods.find((mod) => mod.id === id && mod.image) ?? []);

// Average colour of an image, sampled small. Previews come from
// raw.githubusercontent.com, which sends CORS headers, so the sample is loaded
// anonymously to keep the canvas readable; any failure keeps the default.
const glowCache = new Map<string, string>();
function sampleGlow(src: string): Promise<string | null> {
  const cached = glowCache.get(src);
  if (cached) return Promise.resolve(cached);
  return new Promise((resolve) => {
    const image = new Image();
    image.decoding = "async";
    image.crossOrigin = "anonymous";
    image.onload = () => {
      try {
        const canvas = document.createElement("canvas");
        canvas.width = 12;
        canvas.height = 12;
        const context = canvas.getContext("2d", { willReadFrequently: true });
        if (!context) return resolve(null);
        context.drawImage(image, 0, 0, 12, 12);
        const { data } = context.getImageData(0, 0, 12, 12);
        let red = 0;
        let green = 0;
        let blue = 0;
        let weight = 0;
        for (let index = 0; index < data.length; index += 4) {
          const r = data[index];
          const g = data[index + 1];
          const b = data[index + 2];
          // Saturated pixels say more about a look than grey background.
          const saturation = Math.max(r, g, b) - Math.min(r, g, b);
          const w = 1 + saturation / 24;
          red += r * w;
          green += g * w;
          blue += b * w;
          weight += w;
        }
        const glow = vivid(red / weight, green / weight, blue / weight);
        glowCache.set(src, glow);
        resolve(glow);
      } catch {
        resolve(null);
      }
    };
    image.onerror = () => resolve(null);
    image.src = src;
  });
}

// Keeps the hue of the average colour but makes it a light worth glowing:
// averages of real art are muddy. Near-grey art falls back to the brand violet.
function vivid(red: number, green: number, blue: number) {
  const r = red / 255;
  const g = green / 255;
  const b = blue / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const chroma = max - min;
  if (chroma < 0.06) return "hsl(277 82% 64%)";
  const hue =
    max === r ? ((g - b) / chroma) % 6 : max === g ? (b - r) / chroma + 2 : (r - g) / chroma + 4;
  return `hsl(${Math.round((hue * 60 + 360) % 360)} 78% 62%)`;
}

// Signed distance from the front look, wrapped around the ring.
function offsetOf(index: number, active: number, total: number) {
  let offset = index - active;
  if (offset > total / 2) offset -= total;
  if (offset < -total / 2) offset += total;
  return offset;
}

export default function LookCarousel({
  motion,
  onOpen,
}: {
  motion: boolean;
  onOpen: (mod: StudioMod) => void;
}) {
  const { language, isRu } = useLocale();
  const { animated, visible } = useMotion(motion);
  const list = useMemo(() => looks.filter((mod) => mod.image), []);
  const total = list.length;
  const [active, setActive] = useState(0);
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const [inView, setInView] = useState(true);
  const [dragX, setDragX] = useState(0);
  const [glow, setGlow] = useState<string | null>(null);
  const stage = useRef<HTMLDivElement>(null);
  const drag = useRef<{
    x: number;
    start: number;
    at: number;
    pointer: number;
    moved: boolean;
  } | null>(null);
  const suppressClick = useRef(false);
  const dragging = dragX !== 0;
  const running = animated && visible && inView && !hovered && !focused && !dragging && total > 1;

  const go = useCallback(
    (step: number) => setActive((current) => (((current + step) % total) + total) % total),
    [total],
  );

  useEffect(() => {
    const element = stage.current;
    if (!element) return;
    const observer = new IntersectionObserver(([entry]) => setInView(entry.isIntersecting), {
      threshold: 0.2,
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  // One timer per look in front: changing the look restarts the countdown.
  useEffect(() => {
    if (!running) return;
    const timer = window.setTimeout(() => go(1), AUTO_ADVANCE);
    return () => window.clearTimeout(timer);
  }, [running, active, go]);

  useEffect(() => {
    const src = list[active]?.image;
    if (!src) return;
    let current = true;
    sampleGlow(src).then((value) => {
      if (current && value) setGlow(value);
    });
    return () => {
      current = false;
    };
  }, [active, list]);

  // Card width in pixels, for turning a drag distance into steps.
  const cardWidth = () =>
    stage.current?.querySelector<HTMLElement>(".s-shelf-card.is-front")?.offsetWidth ?? 220;

  const front = list[active];
  if (!front) return null;

  return (
    <div
      className={`s-look-carousel s-shelf ${animated ? "is-animated" : ""} ${running ? "is-running" : ""}`}
      style={glow ? ({ "--shelf-glow": glow } as CSSProperties) : undefined}
      role="region"
      aria-roledescription={isRu ? "карусель" : "carousel"}
      aria-label={isRu ? "Облики из каталога" : "Looks from the catalog"}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onFocus={() => setFocused(true)}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) setFocused(false);
      }}
      onKeyDown={(event) => {
        if (event.key === "ArrowRight") {
          event.preventDefault();
          go(1);
        } else if (event.key === "ArrowLeft") {
          event.preventDefault();
          go(-1);
        }
      }}
    >
      <div className="s-carousel-heading">
        <span>{isRu ? "В ВИТРИНЕ СЕЙЧАС" : "ON THE DISPLAY"}</span>
        <span>
          <b>{String(active + 1).padStart(2, "0")}</b>
          <i>/</i>
          {String(total).padStart(2, "0")} {isRu ? "ОБЛИКОВ" : "LOOKS"}
        </span>
      </div>
      <div
        className={`s-shelf-stage ${dragging ? "is-dragging" : ""}`}
        ref={stage}
        onWheel={(event) => {
          if (Math.abs(event.deltaX) < 24 || Math.abs(event.deltaX) < Math.abs(event.deltaY))
            return;
          const now = Date.now();
          const last = Number(event.currentTarget.dataset.wheelAt ?? 0);
          if (now - last < 420) return;
          event.currentTarget.dataset.wheelAt = String(now);
          go(event.deltaX > 0 ? 1 : -1);
        }}
        onPointerDown={(event) => {
          if (event.button !== 0 || total < 2) return;
          drag.current = {
            x: event.clientX,
            start: event.clientX,
            at: performance.now(),
            pointer: event.pointerId,
            moved: false,
          };
        }}
        onPointerMove={(event) => {
          const state = drag.current;
          if (!state || state.pointer !== event.pointerId) return;
          const delta = event.clientX - state.start;
          if (!state.moved && Math.abs(delta) < 6) return;
          if (!state.moved) {
            state.moved = true;
            event.currentTarget.setPointerCapture(event.pointerId);
          }
          state.x = event.clientX;
          setDragX(delta);
        }}
        onPointerUp={(event) => {
          const state = drag.current;
          drag.current = null;
          if (!state?.moved) return;
          suppressClick.current = true;
          const delta = event.clientX - state.start;
          const elapsed = Math.max(1, performance.now() - state.at);
          // A quick flick counts for more than its distance.
          const velocity = delta / elapsed;
          const steps = Math.round(-(delta + velocity * 180) / (cardWidth() * 0.62));
          setDragX(0);
          if (steps) go(Math.max(-3, Math.min(3, steps)));
        }}
        onPointerCancel={() => {
          drag.current = null;
          setDragX(0);
        }}
        onClickCapture={(event) => {
          if (!suppressClick.current) return;
          suppressClick.current = false;
          event.preventDefault();
          event.stopPropagation();
        }}
      >
        <span className="s-shelf-light" aria-hidden="true" />
        {list.map((mod, index) => {
          const offset = offsetOf(index, active, total);
          const distance = Math.abs(offset);
          const isFront = offset === 0;
          const hidden = distance > VISIBLE_SIDE;
          return (
            <button
              key={mod.id}
              type="button"
              className={`s-shelf-card ${isFront ? "is-front" : ""} ${hidden ? "is-hidden" : ""}`}
              style={
                {
                  "--offset": offset,
                  "--distance": distance,
                  "--drag": `${dragX}px`,
                  zIndex: 10 - distance,
                } as CSSProperties
              }
              tabIndex={isFront ? 0 : -1}
              aria-hidden={hidden || undefined}
              aria-label={
                isFront
                  ? `${isRu ? "Открыть облик" : "Open look"}: ${mod.name[language]}`
                  : `${isRu ? "Показать" : "Show"}: ${mod.name[language]}`
              }
              onClick={() => (isFront ? onOpen(mod) : go(offset))}
              onPointerMove={(event) => {
                if (!isFront || event.pointerType !== "mouse" || dragging) return;
                const rect = event.currentTarget.getBoundingClientRect();
                const x = (event.clientX - rect.left) / rect.width - 0.5;
                const y = (event.clientY - rect.top) / rect.height - 0.5;
                event.currentTarget.style.setProperty("--tilt-x", `${(-y * 7).toFixed(2)}deg`);
                event.currentTarget.style.setProperty("--tilt-y", `${(x * 9).toFixed(2)}deg`);
                event.currentTarget.style.setProperty(
                  "--shine-x",
                  `${((x + 0.5) * 100).toFixed(1)}%`,
                );
              }}
              onPointerLeave={(event) => {
                event.currentTarget.style.removeProperty("--tilt-x");
                event.currentTarget.style.removeProperty("--tilt-y");
                event.currentTarget.style.removeProperty("--shine-x");
              }}
            >
              <Media
                src={mod.image ?? undefined}
                showPending
                pendingLabel={mod.categoryName[language]}
                draggable={false}
              />
              <span className="s-shelf-shine" aria-hidden="true" />
              <span className="s-shelf-caption">
                <small>{mod.categoryName[language]}</small>
                <strong>{mod.name[language]}</strong>
                <ArrowUpRight />
              </span>
            </button>
          );
        })}
        <button
          type="button"
          className="s-shelf-arrow is-prev"
          aria-label={isRu ? "Предыдущий облик" : "Previous look"}
          onClick={() => go(-1)}
        >
          <ChevronLeft />
        </button>
        <button
          type="button"
          className="s-shelf-arrow is-next"
          aria-label={isRu ? "Следующий облик" : "Next look"}
          onClick={() => go(1)}
        >
          <ChevronRight />
        </button>
      </div>
      <div className="s-shelf-dots" role="tablist" aria-label={isRu ? "Облики" : "Looks"}>
        {list.map((mod, index) => (
          <button
            key={mod.id}
            type="button"
            role="tab"
            aria-selected={index === active}
            aria-label={mod.name[language]}
            className={index === active ? "is-active" : ""}
            onClick={() => setActive(index)}
          >
            {index === active && <i key={`${active}-${running}`} />}
          </button>
        ))}
      </div>
      <p className="s-shelf-live" aria-live="polite">
        {front.categoryName[language]} · {front.name[language]}
      </p>
      <div className="s-carousel-footer">
        <span>
          {isRu
            ? "Листай, тяни или жми стрелки · нажми на облик в центре, чтобы открыть"
            : "Swipe, drag or use the arrows · click the look in front to open it"}
        </span>
      </div>
    </div>
  );
}
