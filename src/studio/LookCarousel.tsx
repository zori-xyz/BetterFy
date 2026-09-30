import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowUpRight } from "lucide-react";
import { useLocale } from "../i18n";
import { featuredMods, mods, type StudioMod } from "./model";
import { Media } from "./ui";
import { useMotion } from "./useMotion";

// Each card crosses the stage once, always left to right. Its reset happens
// outside the clipped area, so the visible movement never reverses.
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
    const y = 31 * Math.pow(2 * progress - 1, 2);
    return {
      offset: (progress * travel) / duration,
      transform: `translate3d(${x}px, ${y}px, 0) rotate(${(progress - 0.5) * 5}deg)`,
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
      <div className="s-home-showcase" ref={track}>
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
            ? "Наведи, чтобы рассмотреть · нажми, чтобы открыть"
            : "Hover to inspect · click to open"}
        </span>
      </div>
    </div>
  );
}
