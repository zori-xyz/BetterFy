import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowUpRight } from "lucide-react";
import { useLocale } from "../i18n";
import { featuredMods, mods, type StudioMod } from "./model";
import { Media } from "./ui";
import { useMotion } from "./useMotion";

// Each card crosses the stage once, always left to right. Its reset happens
// outside the clipped area, so the visible movement never reverses. Cards
// grow and come forward as they pass the middle, and the stage can be dragged
// to scrub through the looks.
const duration = 60000;
const travel = 20000;
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

function path(width: number, cardWidth: number): Keyframe[] {
  const moving: Keyframe[] = Array.from({ length: 41 }, (_, index) => {
    const progress = index / 40;
    const x = -cardWidth - 12 + progress * (width + cardWidth + 24);
    const edge = Math.pow(2 * progress - 1, 2);
    const y = 31 * edge;
    const scale = 1.04 - edge * 0.16;
    return {
      offset: (progress * travel) / duration,
      transform: `translate3d(${x}px, ${y}px, 0) rotate(${(progress - 0.5) * 5}deg) scale(${scale})`,
      opacity: progress < 0.035 ? progress / 0.035 : progress > 0.965 ? (1 - progress) / 0.035 : 1,
    };
  });
  return [
    ...moving,
    { offset: 0.999, transform: moving[moving.length - 1].transform, opacity: 0 },
    { offset: 1, transform: moving[0].transform, opacity: 0 },
  ];
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
  const [paused, setPaused] = useState(false);
  const [inView, setInView] = useState(true);
  const track = useRef<HTMLDivElement>(null);
  const animations = useRef<Animation[]>([]);
  const drag = useRef<{ x: number; moved: boolean; pointer: number } | null>(null);
  const suppressClick = useRef(false);
  const running = !paused && visible && inView;
  const list = useMemo(() => looks.filter((mod) => mod.image), []);

  useEffect(() => {
    const observer = new IntersectionObserver(([entry]) => setInView(entry.isIntersecting), {
      threshold: 0.01,
    });
    if (track.current) observer.observe(track.current);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    if (!animated || !track.current || !list.length) return;
    const element = track.current;
    const mount = () => {
      animations.current.forEach((animation) => animation.cancel());
      const cards = [...element.querySelectorAll<HTMLButtonElement>(".s-showcase-look")];
      animations.current = cards.map((card, index) =>
        card.animate(path(element.clientWidth, card.offsetWidth), {
          duration,
          iterations: Infinity,
          delay: -((index * duration) / cards.length),
          easing: "linear",
          fill: "both",
        }),
      );
      if (!running) animations.current.forEach((animation) => animation.pause());
    };
    mount();
    const resize = new ResizeObserver(mount);
    resize.observe(element);
    return () => {
      resize.disconnect();
      animations.current.forEach((animation) => animation.cancel());
      animations.current = [];
    };
  }, [animated, list.length]);
  useEffect(() => {
    animations.current.forEach((animation) => (running ? animation.play() : animation.pause()));
  }, [running, animated]);

  // Dragging the stage moves every card by the same amount of its path.
  const scrub = (deltaX: number) => {
    const element = track.current;
    if (!element) return;
    const card = element.querySelector<HTMLElement>(".s-showcase-look");
    const span = element.clientWidth + (card?.offsetWidth ?? 0) + 24;
    const shift = (deltaX / span) * travel;
    animations.current.forEach((animation) => {
      const time = Number(animation.currentTime ?? 0) + shift;
      animation.currentTime = ((time % duration) + duration) % duration;
    });
  };

  return (
    <div
      className={`s-look-carousel ${animated ? "is-animated" : ""}`}
      role="region"
      aria-label={
        isRu ? "Облики из каталога: движение слева направо" : "Catalog looks moving left to right"
      }
      onMouseEnter={() => setPaused(true)}
      onMouseLeave={() => setPaused(false)}
      onFocus={() => setPaused(true)}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) setPaused(false);
      }}
    >
      <div className="s-carousel-heading">
        <span>{isRu ? "В ВИТРИНЕ СЕЙЧАС" : "ON THE DISPLAY"}</span>
        <span>
          {String(list.length).padStart(2, "0")} {isRu ? "ОБЛИКОВ" : "LOOKS"}
        </span>
      </div>
      <div
        className="s-home-showcase"
        ref={track}
        onPointerDown={(event) => {
          if (!animated || event.button !== 0) return;
          drag.current = { x: event.clientX, moved: false, pointer: event.pointerId };
        }}
        onPointerMove={(event) => {
          const state = drag.current;
          if (!state || state.pointer !== event.pointerId) return;
          const deltaX = event.clientX - state.x;
          if (!state.moved && Math.abs(deltaX) < 6) return;
          if (!state.moved) {
            state.moved = true;
            event.currentTarget.setPointerCapture(event.pointerId);
            event.currentTarget.classList.add("is-dragging");
          }
          state.x = event.clientX;
          scrub(deltaX);
        }}
        onPointerUp={(event) => {
          if (drag.current?.moved) suppressClick.current = true;
          drag.current = null;
          event.currentTarget.classList.remove("is-dragging");
        }}
        onPointerCancel={(event) => {
          drag.current = null;
          event.currentTarget.classList.remove("is-dragging");
        }}
        onClickCapture={(event) => {
          if (!suppressClick.current) return;
          suppressClick.current = false;
          event.preventDefault();
          event.stopPropagation();
        }}
      >
        {list.map((mod) => (
          <button
            key={mod.id}
            className="s-showcase-look"
            onClick={() => onOpen(mod)}
            aria-label={`${isRu ? "Посмотреть облик" : "View look"}: ${mod.name[language]}`}
          >
            <Media
              src={mod.image ?? undefined}
              showPending
              pendingLabel={mod.categoryName[language]}
            />
            <span>
              <small>{mod.categoryName[language]}</small>
              <strong>{mod.name[language]}</strong>
              <ArrowUpRight />
            </span>
          </button>
        ))}
      </div>
      <div className="s-carousel-footer">
        <span>
          {isRu
            ? animated
              ? "Наведи, чтобы рассмотреть · потяни, чтобы прокрутить · нажми, чтобы открыть"
              : "Наведи, чтобы рассмотреть · нажми, чтобы открыть"
            : animated
              ? "Hover to inspect · drag to scroll · click to open"
              : "Hover to inspect · click to open"}
        </span>
      </div>
    </div>
  );
}
