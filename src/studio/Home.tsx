import {
  ArrowRight,
  ArrowUpRight,
  Check,
  Gamepad2,
  Gauge,
  Globe2,
  Layers3,
  LoaderCircle,
  Monitor,
  Paintbrush,
  Plus,
  Send,
  Shirt,
  Sparkles,
  Trees,
  TriangleAlert,
  VolumeX,
  Wrench,
  type LucideIcon,
} from "lucide-react";
import { useLocale, modCount, type Language } from "../i18n";
import { isInstallable, modById, mods, type Domain, type StudioMod } from "./model";
import { bundleRelationProblem, findPackage, packageName } from "./packages";
import { useBuildName } from "./builder/buildName";
import EmoteStage, { EmoteBackdrop, emoteStyle } from "./EmoteStage";
import { useEngineActive } from "./engineActivity";
import { useState } from "react";
import LookCarousel from "./LookCarousel";
import { ModCard } from "./ui";
import { CountUp } from "./delight";
import "./home.css";

type Pair = [ru: string, en: string];

const gameIcons: Record<string, LucideIcon> = {
  core: Gauge,
  world: Trees,
  interface: Monitor,
  audio: VolumeX,
  utility: Wrench,
  custom: Paintbrush,
};
const iconFor = (mod: StudioMod) =>
  mod.domain === "game" ? (gameIcons[mod.category] ?? Gamepad2) : Shirt;

// Dota's default item hotkeys, one per inventory slot.
const hotkeys = ["Z", "X", "C", "V", "B", "N"];
const backpackSlots = 3;
const shelfOrder = ["world", "interface", "core", "audio", "utility", "custom"];

const known = (ids: string[]) => ids.flatMap((id) => (modById.has(id) ? [modById.get(id)!] : []));
const modForPackage = (id: string) => modById.get(findPackage(id)?.catalogId ?? id);
const packageLabel = (id: string, language: Language) =>
  modForPackage(id)?.name[language] ?? packageName(id, language);

export default function Home({
  selected,
  motion,
  installedIds,
  onCatalog,
  onBuild,
  onOpen,
  onToggle,
}: {
  selected: string[];
  motion: boolean;
  /** Packages installed in Dota (catalog or engine IDs); omitted means unknown. */
  installedIds?: string[] | null;
  onCatalog: (domain?: Domain) => void;
  onBuild: () => void;
  onOpen: (mod: StudioMod) => void;
  onToggle: (mod: StudioMod) => void;
}) {
  const { language, isRu } = useLocale();
  const [mood, setMood] = useState(0);
  return (
    <div className="s-home">
      <section className="s-hero" style={emoteStyle(mood)}>
        <EmoteBackdrop active={mood} />
        <div className="s-hero-copy">
          <span className="s-eyebrow">
            <Sparkles />
            {isRu ? "ТВОЯ ИГРА · ТВОЙ ПОЧЕРК" : "YOUR GAME · YOUR SIGNATURE"}
          </span>
          <h1>
            {isRu ? "Собери свою " : "Make Dota "}
            <span className="s-hero-accent">
              {isRu ? "Dota" : "your own"}
              <span className="s-hero-sticker">
                <EmoteStage motion={motion} active={mood} onChange={setMood} />
              </span>
            </span>
          </h1>
          <p>
            {isRu
              ? "Облики любимых героев, эффекты и настройки игры. Всё ставится в Dota и откатывается в один клик."
              : "Hero looks, effects and game settings. Everything installs into Dota and rolls back in one click."}
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
            <CountUp
              value={mods.filter((mod) => isInstallable(mod.id)).length}
              locale={language === "ru" ? "ru-RU" : "en-GB"}
            />
          </span>{" "}
          {isRu ? "модов и обликов в каталоге" : "mods and looks in the catalog"}
        </div>
      </section>
      <BuildDock
        selected={selected}
        installedIds={installedIds ?? null}
        onBuild={onBuild}
        onCatalog={onCatalog}
        onOpen={onOpen}
        onToggle={onToggle}
      />
      <PilotShelf selected={selected} onCatalog={onCatalog} onOpen={onOpen} onToggle={onToggle} />
      <footer className="h-community">
        <p>
          <strong>{isRu ? "Не хватает мода?" : "Missing a mod?"}</strong>{" "}
          {isRu ? "Расскажи нам, добавим." : "Tell us and we'll look into it."}
        </p>
        <nav>
          <a href="https://t.me/BeterHelp" target="_blank" rel="noreferrer">
            <Send />
            Telegram
            <ArrowUpRight />
          </a>
          <a href="https://zori-xyz.github.io/BetterFy/" target="_blank" rel="noreferrer">
            <Globe2 />
            {isRu ? "Сайт BetterFy" : "BetterFy website"}
            <ArrowUpRight />
          </a>
        </nav>
      </footer>
    </div>
  );
}

function SlotArt({ mod }: { mod: StudioMod }) {
  const [failed, setFailed] = useState(false);
  const Icon = iconFor(mod);
  if (!mod.image || failed) return <Icon />;
  return <img src={mod.image} alt="" loading="lazy" onError={() => setFailed(true)} />;
}

type DockState = "empty" | "busy" | "blocked" | "ready" | "changed" | "installed";
// The build as a Dota inventory: six slots with their item hotkeys, then the
// backpack.
function BuildDock({
  selected,
  installedIds,
  onBuild,
  onCatalog,
  onOpen,
  onToggle,
}: {
  selected: string[];
  installedIds: string[] | null;
  onBuild: () => void;
  onCatalog: (domain?: Domain) => void;
  onOpen: (mod: StudioMod) => void;
  onToggle: (mod: StudioMod) => void;
}) {
  const { language, isRu } = useLocale();
  const busy = useEngineActive();
  const name = useBuildName(selected, language);
  // Slots filled before Home opened stay still; new arrivals drop in.
  const [arrived] = useState(() => new Set(selected));
  const chosen = known(selected);
  const pilot = chosen.filter((mod) => isInstallable(mod.id));
  const relation = bundleRelationProblem(pilot.map((mod) => mod.id));
  const required = relation?.kind === "missing" ? modForPackage(relation.otherId) : undefined;
  const installed = installedIds?.length
    ? new Set(installedIds.map((id) => findPackage(id)?.catalogId ?? id))
    : null;
  const inGame = Boolean(
    installed && pilot.length === installed.size && pilot.every((mod) => installed.has(mod.id)),
  );
  const state: DockState = busy
    ? "busy"
    : !pilot.length
      ? "empty"
      : relation
        ? "blocked"
        : inGame
          ? "installed"
          : installed
            ? "changed"
            : "ready";
  const t = (ru: string, en: string) => (isRu ? ru : en);

  let status: { tone: "ready" | "risk" | "quiet" | "busy" | "accent"; text: string } | null = null;
  if (state === "busy")
    status = {
      tone: "busy",
      text: t("BetterFy работает с файлами игры", "BetterFy is working on game files"),
    };
  else if (state === "blocked" && relation) {
    const first = packageLabel(relation.packageId, language);
    const other = packageLabel(relation.otherId, language);
    status = {
      tone: "risk",
      text:
        relation.kind === "missing"
          ? t(
              `Для «${first}» нужен мод «${other}». Без него сборку не поставить.`,
              `${first} needs ${other}. The build won't install without it.`,
            )
          : t(
              `«${first}» и «${other}» не ставятся вместе. Оставь в сборке один.`,
              `${first} and ${other} can't be installed together. Keep one in the build.`,
            ),
    };
  } else if (state === "ready")
    status = {
      tone: "ready",
      text: t(
        `${modCount(pilot.length, language)} можно поставить в Dota`,
        `${modCount(pilot.length, language)} can go into Dota`,
      ),
    };
  else if (state === "changed")
    status = {
      tone: "accent",
      text: t(
        "В Dota стоит прошлая версия сборки. Поставь новую, чтобы изменения появились в игре.",
        "Dota has an earlier version of this build. Install it again to bring the changes in.",
      ),
    };
  else if (state === "installed")
    status = {
      tone: "ready",
      text: t(
        `Сборка стоит в Dota · ${modCount(pilot.length, language)}`,
        `This build is in Dota · ${modCount(pilot.length, language)}`,
      ),
    };
  else if (installed)
    status = {
      tone: "accent",
      text: t(
        `В Dota ещё стоит прошлая сборка: ${modCount(installed.size, language)}.`,
        `An earlier build is still in Dota: ${modCount(installed.size, language)}.`,
      ),
    };

  let action: { label: string; run: () => void } | null = null;
  if (state === "busy") action = { label: t("Смотреть прогресс", "See progress"), run: onBuild };
  else if (state === "blocked" && required)
    action = {
      label: t(`Добавить «${required.name.ru}»`, `Add ${required.name.en}`),
      run: () => onToggle(required),
    };
  else if (state === "ready" || state === "changed")
    action = { label: t("Перейти к установке", "Go to install"), run: onBuild };
  else if (state !== "empty" || installed)
    action = { label: t("Открыть сборку", "Open build"), run: onBuild };

  const visible = hotkeys.length + backpackSlots;
  const overflow = chosen.length > visible ? chosen.length - visible + 1 : 0;

  const slot = (index: number, hotkey?: string) => {
    const mod = chosen[index];
    if (overflow && index === visible - 1)
      return (
        <button
          key="more"
          className="h-slot is-more"
          onClick={onBuild}
          aria-label={t(`Ещё ${overflow} в сборке`, `${overflow} more in the build`)}
        >
          +{overflow}
        </button>
      );
    if (mod)
      return (
        <button
          key={mod.id}
          className={`h-slot is-filled ${arrived.has(mod.id) ? "" : "is-new"}`}
          onClick={() => onOpen(mod)}
          title={mod.name[language]}
          aria-label={`${t("Подробнее", "Details")}: ${mod.name[language]}`}
        >
          <SlotArt mod={mod} />
          {hotkey && <kbd aria-hidden="true">{hotkey}</kbd>}
        </button>
      );
    if (index === chosen.length)
      return (
        <button
          key="next"
          className="h-slot is-next"
          onClick={() => onCatalog()}
          aria-label={t("Свободный слот: выбрать мод в каталоге", "Free slot: pick a mod")}
          title={t("Свободный слот", "Free slot")}
        >
          <Plus />
          {hotkey && <kbd aria-hidden="true">{hotkey}</kbd>}
        </button>
      );
    return (
      <span key={`empty-${index}`} className="h-slot" aria-hidden="true">
        {hotkey && <kbd>{hotkey}</kbd>}
      </span>
    );
  };

  return (
    <section className={`h-dock is-${state}`} aria-labelledby="h-dock-title">
      <div className="h-dock-copy">
        <span className="s-eyebrow">
          <Layers3 />
          {isRu ? "ТВОЯ СБОРКА" : "YOUR BUILD"}
          {chosen.length > 0 && ` · ${modCount(chosen.length, language).toUpperCase()}`}
        </span>
        <h2 id="h-dock-title">
          {state === "empty" ? t("Инвентарь пуст", "Empty inventory") : name.name}
        </h2>
        <p>
          {state === "empty"
            ? t(
                "Каждый мод и облик займёт слот, как предмет в Dota. Начни с каталога или с модов ниже.",
                "Every mod and look takes a slot, like an item in Dota. Start in the catalog or with the mods below.",
              )
            : name.summary}
        </p>
        {status && (
          <p className={`h-dock-status is-${status.tone}`} role="status">
            {status.tone === "busy" ? (
              <LoaderCircle className="s-spin" />
            ) : status.tone === "risk" ? (
              <TriangleAlert />
            ) : state === "installed" ? (
              <Check />
            ) : (
              <i aria-hidden="true" />
            )}
            <span>{status.text}</span>
          </p>
        )}
      </div>
      <div className="h-inv" role="group" aria-label={t("Слоты сборки", "Build slots")}>
        <div className="h-inv-main">{hotkeys.map((key, index) => slot(index, key))}</div>
        <div className="h-inv-pack" title={t("Рюкзак", "Backpack")}>
          <span className="h-visually-hidden">{t("Рюкзак", "Backpack")}</span>
          {Array.from({ length: backpackSlots }, (_, index) => slot(hotkeys.length + index))}
        </div>
      </div>
      {action && (
        <div className="h-dock-actions">
          <button className="s-btn s-btn-primary" onClick={action.run}>
            <span>{action.label}</span>
            {state === "blocked" && required ? <Plus /> : <ArrowRight />}
          </button>
        </div>
      )}
    </section>
  );
}

function PilotShelf({
  selected,
  onCatalog,
  onOpen,
  onToggle,
}: {
  selected: string[];
  onCatalog: (domain?: Domain) => void;
  onOpen: (mod: StudioMod) => void;
  onToggle: (mod: StudioMod) => void;
}) {
  const { language, isRu } = useLocale();
  const pilots = mods.filter((mod) => mod.domain === "game" && isInstallable(mod.id));
  const groups = shelfOrder
    .map((id) => ({ id, items: pilots.filter((mod) => mod.category === id) }))
    .filter((group) => group.items.length > 0);
  const [tab, setTab] = useState(groups[0]?.id ?? "");
  const active = groups.find((group) => group.id === tab) ?? groups[0];
  if (!active) return null;
  const picked = new Set(selected);
  // The same cards and tabs as the catalog, so Home reads as the same app.
  return (
    <section className="h-section h-pilot">
      <header className="s-section-head">
        <div>
          <span className="s-eyebrow">{isRu ? "НАСТРОЙКА ИГРЫ" : "GAME TUNING"}</span>
          <h2>{isRu ? "Настрой саму Dota" : "Tune Dota itself"}</h2>
        </div>
        <button className="s-text-button" onClick={() => onCatalog("game")}>
          {isRu ? "Все настройки" : "All tuning mods"}
          <ArrowRight />
        </button>
      </header>
      <div
        className="s-domain-tabs h-pilot-tabs"
        role="tablist"
        aria-label={isRu ? "Категории" : "Categories"}
      >
        {groups.map((group) => (
          <button
            key={group.id}
            role="tab"
            aria-selected={group.id === active.id}
            onClick={() => setTab(group.id)}
          >
            {group.items[0].categoryName[language]}
          </button>
        ))}
      </div>
      <div className="s-mod-grid h-pilot-grid" role="tabpanel" key={active.id}>
        {active.items.slice(0, 4).map((mod) => (
          <ModCard
            key={mod.id}
            mod={mod}
            selected={picked.has(mod.id)}
            onOpen={() => onOpen(mod)}
            onToggle={() => onToggle(mod)}
          />
        ))}
      </div>
    </section>
  );
}
