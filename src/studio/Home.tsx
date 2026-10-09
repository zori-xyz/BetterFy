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
import { presetTitle, workshopPresets, type BetterFyPreset } from "../presets";
import { deliveryLabel, isPilotMod, modById, mods, type Domain, type StudioMod } from "./model";
import { bundleRelationProblem, findPackage, packageName } from "./packages";
import { useBuildName } from "./builder/buildName";
import { useEngineActive } from "./engineActivity";
import EmoteStage, { emoteStyle } from "./EmoteStage";
import { useRef, useState, type CSSProperties, type KeyboardEvent } from "react";
import LookCarousel from "./LookCarousel";
import { CountUp } from "./delight";
import "./home.css";

type Pair = [ru: string, en: string];

// What each built-in set changes in the match, in one line. The list of mods
// under it comes from the preset itself.
const setCopy: Record<string, { kicker: Pair; line: Pair; icon: LucideIcon }> = {
  "betterfy.focus-performance": {
    kicker: ["Производительность", "Performance"],
    line: [
      "Карта без кустов, реки и погоды, меньше эффектов на зданиях.",
      "A map without bushes, river or weather, and fewer effects on buildings.",
    ],
    icon: Gauge,
  },
  "betterfy.clean-interface": {
    kicker: ["Интерфейс", "Interface"],
    line: [
      "Прозрачный HUD, крупные карточки героев и свой Net Worth на виду.",
      "A transparent HUD, bigger hero cards and your net worth in view.",
    ],
    icon: Monitor,
  },
  "betterfy.quiet-match": {
    kicker: ["Звук", "Audio"],
    line: [
      "Без звуков окружения, насмешек и реплик героев.",
      "No ambience, taunts or hero voice lines.",
    ],
    icon: VolumeX,
  },
};

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
// Bar heights for the sets that have no preview art (the audio set).
const bars = [0.35, 0.6, 0.45, 0.8, 0.55, 0.95, 0.7, 0.5, 0.85, 0.4, 0.65, 0.3, 0.75, 0.5, 0.6];
const shelfOrder = ["world", "interface", "core", "audio", "utility", "custom"];

const known = (ids: string[]) => ids.flatMap((id) => (modById.has(id) ? [modById.get(id)!] : []));
const modForPackage = (id: string) => modById.get(findPackage(id)?.catalogId ?? id);
const packageLabel = (id: string, language: Language) =>
  modForPackage(id)?.name[language] ?? packageName(id, language);

/** Requirements and exclusions from the package manifest, as catalog mods. */
function relationsOf(mod: StudioMod) {
  const manifest = findPackage(mod.id);
  const resolve = (ids: string[] | undefined) =>
    (ids ?? []).flatMap((id) => {
      const other = findPackage(id);
      // A superseded package, or the mod's own earlier version, is no choice.
      if (!other || other.supersededBy || other.catalogId === mod.id) return [];
      const target = modById.get(other.catalogId);
      return target ? [target] : [];
    });
  return { needs: resolve(manifest?.requires), clashes: resolve(manifest?.conflicts) };
}

export default function Home({
  selected,
  motion,
  installedIds,
  onCatalog,
  onBuild,
  onOpen,
  onToggle,
  onCollection,
}: {
  selected: string[];
  motion: boolean;
  /** Packages installed in Dota (catalog or engine IDs); omitted means unknown. */
  installedIds?: string[] | null;
  onCatalog: (domain?: Domain) => void;
  onBuild: () => void;
  onOpen: (mod: StudioMod) => void;
  onToggle: (mod: StudioMod) => void;
  onCollection: (preset: BetterFyPreset) => void;
}) {
  const { language, isRu } = useLocale();
  const [mood, setMood] = useState(0);
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
      <BuildDock
        selected={selected}
        installedIds={installedIds ?? null}
        onBuild={onBuild}
        onCatalog={onCatalog}
        onOpen={onOpen}
        onToggle={onToggle}
      />
      <ReadySets selected={selected} onCollection={onCollection} />
      <PilotShelf selected={selected} onCatalog={onCatalog} onOpen={onOpen} onToggle={onToggle} />
      <section className="h-community">
        <div>
          <span className="s-eyebrow">{isRu ? "СООБЩЕСТВО BETTERFY" : "BETTERFY COMMUNITY"}</span>
          <h2>{isRu ? "Не хватает мода? Расскажи нам" : "Missing a mod? Tell us"}</h2>
          <p>
            {isRu
              ? "Идеи и вопросы — в Telegram. История BetterFy и материалы о продукте — на сайте."
              : "Ideas and questions go to Telegram. BetterFy's story and product notes live on the website."}
          </p>
        </div>
        <div className="h-community-links">
          <a className="s-btn" href="https://t.me/BeterHelp" target="_blank" rel="noreferrer">
            <Send />
            Telegram
            <code>@BeterHelp</code>
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

function SlotArt({ mod }: { mod: StudioMod }) {
  const [failed, setFailed] = useState(false);
  const Icon = iconFor(mod);
  if (!mod.image || failed) return <Icon />;
  return <img src={mod.image} alt="" loading="lazy" onError={() => setFailed(true)} />;
}

type DockState = "empty" | "busy" | "blocked" | "preview" | "ready" | "changed" | "installed";
type StepState = "done" | "current" | "busy" | "todo" | "off";

const laneByState: Record<DockState, StepState[]> = {
  empty: ["current", "todo", "todo", "todo"],
  preview: ["done", "off", "off", "off"],
  blocked: ["done", "current", "todo", "todo"],
  ready: ["done", "current", "todo", "todo"],
  changed: ["done", "current", "todo", "todo"],
  busy: ["done", "busy", "busy", "todo"],
  installed: ["done", "done", "done", "current"],
};

// The build as a Dota inventory: six slots with their item hotkeys, then the
// backpack. Under it, where the build is on its way into the game.
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
  const pilot = chosen.filter((mod) => mod.domain === "game" && isPilotMod(mod.id));
  const previewCount = chosen.length - pilot.length;
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
    : !chosen.length
      ? "empty"
      : relation
        ? "blocked"
        : !pilot.length
          ? "preview"
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
  } else if (state === "preview")
    status = {
      tone: "quiet",
      text: t(
        "Пока только превью: в Dota ставятся моды с отметкой «Windows-пилот».",
        "Preview only for now: only mods marked Windows pilot install into Dota.",
      ),
    };
  else if (state === "ready")
    status = {
      tone: "ready",
      text:
        t(
          `${modCount(pilot.length, language)} можно поставить в Dota`,
          `${modCount(pilot.length, language)} can go into Dota`,
        ) +
        (previewCount
          ? t(` · ${previewCount} только превью`, ` · ${previewCount} preview only`)
          : ""),
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

  const steps: Pair[] = [
    ["Выбор", "Pick"],
    ["Сверка файлов", "File check"],
    ["Запись с бэкапом", "Backed-up write"],
    ["Игра", "Play"],
  ];
  const lane = laneByState[state];
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
                "Каждый мод и облик займёт слот, как предмет в Dota. Возьми готовый набор ниже или добавь моды по одному.",
                "Every mod and look takes a slot, like an item in Dota. Take a ready-made set below or add mods one by one.",
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
        {action && (
          <div className="h-dock-actions">
            <button className="s-btn s-btn-primary" onClick={action.run}>
              <span>{action.label}</span>
              {state === "blocked" && required ? <Plus /> : <ArrowRight />}
            </button>
          </div>
        )}
      </div>
      <div className="h-inv" role="group" aria-label={t("Слоты сборки", "Build slots")}>
        <div className="h-inv-main">{hotkeys.map((key, index) => slot(index, key))}</div>
        <span className="h-inv-label" aria-hidden="true">
          {t("Рюкзак", "Backpack")}
        </span>
        <div className="h-inv-pack">
          {Array.from({ length: backpackSlots }, (_, index) => slot(hotkeys.length + index))}
        </div>
      </div>
      <ol className="h-lane" aria-label={t("Путь сборки в игру", "The build's way into the game")}>
        {steps.map((step, index) => (
          <li
            key={step[1]}
            className={`is-${lane[index]}`}
            aria-current={lane[index] === "current" ? "step" : undefined}
          >
            <span className="h-lane-mark" aria-hidden="true">
              {lane[index] === "done" ? (
                <Check />
              ) : lane[index] === "busy" ? (
                <LoaderCircle className="s-spin" />
              ) : (
                index + 1
              )}
            </span>
            <span>{isRu ? step[0] : step[1]}</span>
          </li>
        ))}
      </ol>
    </section>
  );
}

function ReadySets({
  selected,
  onCollection,
}: {
  selected: string[];
  onCollection: (preset: BetterFyPreset) => void;
}) {
  const { language, isRu } = useLocale();
  const chosen = new Set(selected);
  return (
    <section className="h-section">
      <header className="s-section-head">
        <div>
          <span className="s-eyebrow">{isRu ? "ГОТОВЫЕ НАБОРЫ BETTERFY" : "BETTERFY SETS"}</span>
          <h2>{isRu ? "Возьми набор целиком" : "Take a whole set"}</h2>
        </div>
        <span>
          {isRu ? "Любой мод из набора потом можно убрать" : "Drop any mod from a set later"}
        </span>
      </header>
      <div className="h-sets">
        {workshopPresets.map((preset) => {
          const ids = [...preset.modIds, ...preset.wardrobeIds];
          const items = known(ids);
          const have = ids.filter((id) => chosen.has(id)).length;
          const complete = ids.length > 0 && have === ids.length;
          const allPilot =
            items.length === ids.length &&
            items.every((mod) => mod.domain === "game" && isPilotMod(mod.id));
          const copy = setCopy[preset.id];
          const Icon = copy?.icon ?? Layers3;
          const art = items.flatMap((mod) => (mod.image ? [mod.image] : [])).slice(0, 3);
          const title = presetTitle(preset, language);
          return (
            <article
              className={`h-set ${complete ? "is-complete" : ""}`}
              key={preset.id}
              data-spotlight
            >
              <div className="h-set-art" aria-hidden="true">
                {art.length ? (
                  <div className="h-set-fan">
                    {art.map((src, index) => {
                      const offset = index - (art.length - 1) / 2;
                      return (
                        <span
                          key={src}
                          style={
                            {
                              "--o": offset,
                              "--a": Math.abs(offset),
                              zIndex: 3 - Math.abs(offset),
                            } as CSSProperties
                          }
                        >
                          <img src={src} alt="" loading="lazy" />
                        </span>
                      );
                    })}
                  </div>
                ) : (
                  <div className="h-set-bars">
                    {bars.map((height, index) => (
                      <i key={index} style={{ "--h": height, "--d": index } as CSSProperties} />
                    ))}
                  </div>
                )}
                <span className="h-set-icon">
                  <Icon />
                </span>
              </div>
              <div className="h-set-body">
                <span className="h-set-kicker">
                  {copy ? copy.kicker[isRu ? 0 : 1] : modCount(ids.length, language)}
                </span>
                <h3>{title}</h3>
                {copy && <p>{copy.line[isRu ? 0 : 1]}</p>}
                <ul className="h-set-list" aria-label={isRu ? "Что внутри" : "Inside"}>
                  {items.map((mod) => (
                    <li key={mod.id} className={chosen.has(mod.id) ? "is-in" : ""}>
                      {chosen.has(mod.id) ? <Check aria-hidden="true" /> : <i aria-hidden="true" />}
                      <span>{mod.name[language]}</span>
                      {chosen.has(mod.id) && (
                        <span className="h-visually-hidden">
                          {isRu ? ", в сборке" : ", in your build"}
                        </span>
                      )}
                    </li>
                  ))}
                </ul>
                <footer>
                  <span className="h-set-meta">
                    {complete
                      ? isRu
                        ? "Весь набор в сборке"
                        : "All of it is in your build"
                      : have
                        ? isRu
                          ? `${have} из ${ids.length} уже в сборке`
                          : `${have} of ${ids.length} already in your build`
                        : modCount(ids.length, language)}
                  </span>
                  {allPilot && (
                    <span
                      className="h-pilot-tag"
                      title={
                        isRu
                          ? "Все моды набора доступны для проверяемой установки в Windows-пилоте"
                          : "Every mod in this set is available for verifiable installation in the Windows pilot"
                      }
                    >
                      {deliveryLabel("pilot", language)}
                    </span>
                  )}
                  <button className="h-set-open" onClick={() => onCollection(preset)}>
                    <span className="h-visually-hidden">{title}: </span>
                    {complete
                      ? isRu
                        ? "Посмотреть"
                        : "View"
                      : isRu
                        ? "Открыть набор"
                        : "Open set"}
                    <ArrowRight />
                  </button>
                </footer>
              </div>
            </article>
          );
        })}
      </div>
    </section>
  );
}

// The packages the Windows pilot installs, by category, with quick add.
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
  const pilots = mods.filter((mod) => mod.domain === "game" && isPilotMod(mod.id));
  const groups = shelfOrder
    .map((id) => ({ id, items: pilots.filter((mod) => mod.category === id) }))
    .filter((group) => group.items.length > 0);
  const [tab, setTab] = useState(groups[0]?.id ?? "");
  const tabs = useRef<Record<string, HTMLButtonElement | null>>({});
  const active = groups.find((group) => group.id === tab) ?? groups[0];
  if (!active) return null;
  const picked = new Set(selected);
  const move = (event: KeyboardEvent, index: number) => {
    const last = groups.length - 1;
    const next =
      event.key === "ArrowRight"
        ? index === last
          ? 0
          : index + 1
        : event.key === "ArrowLeft"
          ? index === 0
            ? last
            : index - 1
          : event.key === "Home"
            ? 0
            : event.key === "End"
              ? last
              : -1;
    if (next < 0) return;
    event.preventDefault();
    setTab(groups[next].id);
    tabs.current[groups[next].id]?.focus();
  };
  return (
    <section className="h-section h-pilot">
      <header className="s-section-head">
        <div>
          <span className="s-eyebrow">
            {deliveryLabel("pilot", language)} · {modCount(pilots.length, language).toUpperCase()}
          </span>
          <h2>{isRu ? "Моды, которые можно поставить" : "Mods you can install"}</h2>
        </div>
        <button className="s-text-button" onClick={() => onCatalog("game")}>
          {isRu ? "Все моды для игры" : "All game mods"}
          <ArrowRight />
        </button>
      </header>
      <p className="h-pilot-lead">
        {isRu
          ? "Только эти пакеты BetterFy сверяет и записывает в Dota: одной сборкой, с бэкапом исходного файла. Облики и остальной каталог пока только в превью."
          : "These are the only packages BetterFy checks and writes into Dota: one build, with a backup of the original file. Looks and the rest of the catalog are preview only for now."}
      </p>
      <div className="h-tabs" role="tablist" aria-label={isRu ? "Категории" : "Categories"}>
        {groups.map((group, index) => {
          const Icon = gameIcons[group.id] ?? Gamepad2;
          const count = group.items.filter((mod) => picked.has(mod.id)).length;
          const current = group.id === active.id;
          return (
            <button
              key={group.id}
              ref={(element) => {
                tabs.current[group.id] = element;
              }}
              role="tab"
              id={`h-tab-${group.id}`}
              aria-selected={current}
              aria-controls="h-pilot-panel"
              tabIndex={current ? 0 : -1}
              onClick={() => setTab(group.id)}
              onKeyDown={(event) => move(event, index)}
            >
              <Icon />
              <span>{group.items[0].categoryName[language]}</span>
              <small
                className={count ? "is-some" : ""}
                aria-label={
                  count
                    ? isRu
                      ? `${count} из ${group.items.length} в сборке`
                      : `${count} of ${group.items.length} in your build`
                    : modCount(group.items.length, language)
                }
              >
                {count ? `${count}/${group.items.length}` : group.items.length}
              </small>
            </button>
          );
        })}
      </div>
      <div
        className="h-items"
        role="tabpanel"
        id="h-pilot-panel"
        aria-labelledby={`h-tab-${active.id}`}
        key={active.id}
      >
        {active.items.map((mod) => {
          const inBuild = picked.has(mod.id);
          const { needs, clashes } = relationsOf(mod);
          const missing = needs.filter((other) => !picked.has(other.id));
          const clashing = clashes.filter((other) => picked.has(other.id));
          return (
            <article className={`h-item ${inBuild ? "is-in" : ""}`} key={mod.id}>
              <button className="h-item-main" onClick={() => onOpen(mod)}>
                <span className="h-item-thumb">
                  <SlotArt mod={mod} />
                </span>
                <span className="h-item-text">
                  <strong>{mod.name[language]}</strong>
                  <small>{mod.description[language].split("\n")[0]}</small>
                  {needs.map((other) => (
                    <em
                      key={other.id}
                      className={inBuild && missing.includes(other) ? "is-risk" : ""}
                    >
                      {isRu ? `нужен «${other.name.ru}»` : `needs ${other.name.en}`}
                    </em>
                  ))}
                  {clashes.map((other) => (
                    <em
                      key={other.id}
                      className={inBuild && clashing.includes(other) ? "is-risk" : ""}
                    >
                      {isRu ? `не вместе с «${other.name.ru}»` : `not with ${other.name.en}`}
                    </em>
                  ))}
                </span>
              </button>
              <button
                className={`s-add ${inBuild ? "is-selected" : ""}`}
                onClick={() => onToggle(mod)}
                aria-label={`${inBuild ? (isRu ? "Убрать" : "Remove") : isRu ? "Добавить" : "Add"}: ${mod.name[language]}`}
                aria-pressed={inBuild}
              >
                {inBuild ? <Check /> : <Plus />}
              </button>
            </article>
          );
        })}
      </div>
    </section>
  );
}
