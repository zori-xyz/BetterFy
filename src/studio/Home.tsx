import {
  ArrowRight,
  ArrowUpRight,
  Check,
  Gamepad2,
  Gauge,
  Layers3,
  LoaderCircle,
  Monitor,
  Paintbrush,
  Plus,
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
import { BotStage } from "./BotStage";
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
    <div className="s-home" style={emoteStyle(mood)}>
      <section className="s-hero">
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
      <BotStage motion={motion} />
      <BuildNudge
        selected={selected}
        installedIds={installedIds ?? null}
        onBuild={onBuild}
        onToggle={onToggle}
      />
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
function BuildNudge({
  selected,
  installedIds,
  onBuild,
  onToggle,
}: {
  selected: string[];
  installedIds: string[] | null;
  onBuild: () => void;
  onToggle: (mod: StudioMod) => void;
}) {
  const { language, isRu } = useLocale();
  const busy = useEngineActive();
  const name = useBuildName(selected, language);
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

  // Nothing to remind about: an empty build, or one already in Dota.
  if (state === "empty" || state === "installed") return null;
  const title = {
    busy: t("Идёт установка", "Installing"),
    blocked: t("В сборке конфликт", "The build has a conflict"),
    changed: t("Сборка изменилась", "The build changed"),
    ready: t("Сборка ждёт установки", "Your build is waiting"),
  }[state];
  const line =
    state === "ready"
      ? `${name.name} · ${modCount(chosen.length, language)}`
      : (status?.text ?? name.name);
  const run = action?.run ?? onBuild;

  // A floating reminder at the bottom of Home, only while the build is not in
  // the game: a fan of its first looks, what is left to do, one arrow.
  return (
    <button
      type="button"
      className={`h-nudge is-${state}`}
      onClick={run}
      aria-label={`${title}. ${line}. ${action?.label ?? ""}`}
    >
      <span className="h-nudge-fan" aria-hidden="true">
        {chosen.slice(0, 3).map((mod, index) => (
          <span key={mod.id} style={{ ["--i" as string]: index }}>
            <SlotArt mod={mod} />
          </span>
        ))}
        {chosen.length > 3 && <b>+{chosen.length - 3}</b>}
      </span>
      <span className="h-nudge-copy">
        <strong>
          {state === "busy" ? (
            <LoaderCircle className="s-spin" />
          ) : state === "blocked" ? (
            <TriangleAlert />
          ) : (
            <i aria-hidden="true" />
          )}
          {title}
        </strong>
        <small>{line}</small>
      </span>
      <span className="h-nudge-go" aria-hidden="true">
        {state === "blocked" && required ? <Plus /> : <ArrowRight />}
      </span>
    </button>
  );
}
