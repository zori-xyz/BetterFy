import {
  ArrowRight,
  ArrowUpRight,
  Gamepad2,
  Gauge,
  Layers3,
  Monitor,
  Volume2,
  Sparkles,
  Globe2,
  Send,
} from "lucide-react";
import { useLocale, modCount } from "../i18n";
import { workshopPresets, type BetterFyPreset } from "../presets";
import { featuredMods, modById, mods, type Domain, type StudioMod } from "./model";
import { ModCard } from "./ui";
import EmoteStage, { emoteStyle } from "./EmoteStage";
import { useState } from "react";
import LookCarousel from "./LookCarousel";
import { CountUp } from "./delight";

export const collectionCopy = [
  {
    ru: "Убрать лишние эффекты",
    en: "Remove extra effects",
    shortRu: "Производительность",
    shortEn: "Performance",
    ruText: "Убери тяжёлые детали карты и фоновые эффекты.",
    enText: "Remove demanding world details and background effects.",
    icon: Gauge,
    color: "sage",
    image: "minify-remove-foilage",
  },
  {
    ru: "Прозрачный HUD и меню",
    en: "Transparent HUD & menus",
    shortRu: "Интерфейс",
    shortEn: "Interface",
    ruText: "Прозрачный HUD и компактная сетка героев.",
    enText: "A transparent HUD and a compact hero grid.",
    icon: Monitor,
    color: "sand",
    image: "minify-transparent-hud",
  },
  {
    ru: "Настроить звуки и реплики",
    en: "Adjust sounds & voices",
    shortRu: "Звук",
    shortEn: "Audio",
    ruText: "Настрой звуки окружения, реплики и насмешки.",
    enText: "Choose the ambience, voices and taunts you hear.",
    icon: Volume2,
    color: "blue",
    image: "",
  },
];

export default function Home({
  selected,
  motion,
  onCatalog,
  onBuild,
  onOpen,
  onToggle,
  onCollection,
}: {
  selected: string[];
  motion: boolean;
  onCatalog: (domain?: Domain) => void;
  onBuild: () => void;
  onOpen: (mod: StudioMod) => void;
  onToggle: (mod: StudioMod) => void;
  onCollection: (preset: BetterFyPreset) => void;
}) {
  const { language, isRu } = useLocale();
  const [mood, setMood] = useState(0);
  const chosen = selected.flatMap((id) => (modById.has(id) ? [modById.get(id)!] : []));
  return (
    <div className="s-home">
      <section className="s-hero" style={emoteStyle(mood)}>
        <div className="s-hero-copy">
          <div className="s-hero-mood">
            <EmoteStage motion={motion} active={mood} onChange={setMood} />
            <div>
              <span className="s-eyebrow">
                <Sparkles />
                {isRu ? "ТВОЯ ИГРА · ТВОЙ ПОЧЕРК" : "YOUR GAME · YOUR SIGNATURE"}
              </span>
              <span className="s-hero-mood-note">
                {isRu
                  ? "Начни с детали. Собери целый мир."
                  : "Start with a detail. Shape the whole world."}
              </span>
            </div>
          </div>
          <span className="s-eyebrow">
            <Gamepad2 />
            {isRu ? "ОБЛИКИ · ЭФФЕКТЫ · НАСТРОЙКИ" : "LOOKS · EFFECTS · SETTINGS"}
          </span>
          <h1>
            {isRu ? (
              <>
                Собери свою <span>Dota</span>
              </>
            ) : (
              <>
                Make Dota <span>your own</span>
              </>
            )}
          </h1>
          <p>
            {isRu
              ? "Облики любимых героев, эффекты и настройки игры. Выбирай по одному или начни с готового набора."
              : "Hero looks, effects and game settings. Pick them individually or start with a ready-made set."}
          </p>
          <div className="s-hero-actions">
            <button className="s-btn s-btn-primary" onClick={() => onCatalog("wardrobe")}>
              {isRu ? "Открыть каталог обликов" : "Explore hero looks"}
              <ArrowRight />
            </button>
            <button className="s-btn s-btn-ghost" onClick={() => onCatalog("game")}>
              {isRu ? "Настроить игру" : "Tune your game"}
              <ArrowUpRight />
            </button>
          </div>
        </div>
        <LookCarousel motion={motion} onOpen={onOpen} />
        <div className="s-hero-inventory">
          <span>
            <CountUp value={mods.length} locale={language === "ru" ? "ru-RU" : "en-GB"} />
          </span>{" "}
          {isRu ? "модов и обликов в каталоге" : "mods and looks in the catalog"}
        </div>
      </section>
      {chosen.length > 0 && (
        <section className="s-resume">
          <span className="s-resume-symbol">
            <Layers3 />
          </span>
          <div className="s-resume-copy">
            <span className="s-eyebrow">{isRu ? "ТЕКУЩИЙ НАБОР" : "CURRENT BUILD"}</span>
            <strong>{isRu ? "Твой набор уже начат" : "Your build is taking shape"}</strong>
            <span>
              {modCount(chosen.length, language)} ·{" "}
              {isRu ? "выбор сохранён локально" : "selection saved locally"}
            </span>
          </div>
          <div className="s-resume-images" aria-hidden="true">
            {chosen
              .filter((mod) => mod.image)
              .slice(0, 3)
              .map((mod) => (
                <span key={mod.id}>
                  <img src={mod.image!} alt="" />
                </span>
              ))}
          </div>
          <button className="s-btn" onClick={onBuild}>
            {isRu ? "Продолжить" : "Continue"}
            <ArrowRight />
          </button>
        </section>
      )}
      <section className="s-home-section">
        <header className="s-section-head">
          <div>
            <span className="s-eyebrow">BETTERFY PICKS</span>
            <h2>{isRu ? "Начни с готового набора" : "A starting point, picked for you"}</h2>
          </div>
          <span>{isRu ? "Каждый мод можно изменить" : "Make every choice your own"}</span>
        </header>
        <div className="s-collection-grid">
          {workshopPresets.map((preset, index) => {
            const item = collectionCopy[index];
            const Icon = item.icon;
            const preview = [...preset.modIds, ...preset.wardrobeIds]
              .map((id) => modById.get(id)?.image)
              .find(Boolean);
            return (
              <button
                className={`s-collection-card tone-${item.color}`}
                key={preset.id}
                onClick={() => onCollection(preset)}
              >
                <div className="s-collection-art">
                  {preview && <img src={preview} alt="" loading="lazy" />}
                  <span>
                    <Icon />
                  </span>
                </div>
                <div className="s-collection-copy">
                  <span>{isRu ? item.shortRu : item.shortEn}</span>
                  <h3>{isRu ? item.ru : item.en}</h3>
                  <footer>
                    <span>
                      {modCount(preset.modIds.length + preset.wardrobeIds.length, language)}
                    </span>
                    <ArrowUpRight />
                  </footer>
                </div>
              </button>
            );
          })}
        </div>
      </section>
      <section className="s-home-section">
        <header className="s-section-head">
          <div>
            <span className="s-eyebrow">{isRu ? "ОБЛИКИ И ЭФФЕКТЫ" : "LOOKS & EFFECTS"}</span>
            <h2>{isRu ? "Присмотрись к этим" : "Worth a closer look"}</h2>
          </div>
          <button className="s-text-button" onClick={() => onCatalog("wardrobe")}>
            {isRu ? "Весь каталог" : "Browse all"}
            <ArrowRight />
          </button>
        </header>
        <div className="s-featured-grid">
          {featuredMods.slice(0, 4).map((mod) => (
            <ModCard
              key={mod.id}
              mod={mod}
              selected={selected.includes(mod.id)}
              onOpen={() => onOpen(mod)}
              onToggle={() => onToggle(mod)}
            />
          ))}
        </div>
      </section>
      <section className="s-community-strip">
        <div>
          <span className="s-eyebrow">BETTERFY COMMUNITY</span>
          <h3>{isRu ? "Есть идея для следующего мода?" : "An idea for the next mod?"}</h3>
          <p>
            {isRu
              ? "Предложения, помощь и новости проекта — в Telegram. История BetterFy и материалы о продукте — на сайте."
              : "Ideas, help and project updates live on Telegram. Discover BetterFy and its story on the website."}
          </p>
        </div>
        <div className="s-community-links">
          <a className="s-btn" href="https://t.me/BeterFyBot" target="_blank" rel="noreferrer">
            <Send />
            {isRu ? "Telegram" : "Telegram"}
            <ArrowUpRight />
          </a>
          <a
            className="s-btn"
            href="https://zori-xyz.github.io/BetterFy/"
            target="_blank"
            rel="noreferrer"
          >
            <Globe2 />
            {isRu ? "Сайт BetterFy" : "BetterFy website"}
            <ArrowUpRight />
          </a>
        </div>
      </section>
    </div>
  );
}
