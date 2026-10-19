import { useMemo, useState, type Dispatch, type SetStateAction } from "react";
import {
  ArrowRight,
  Check,
  CheckCheck,
  ChevronDown,
  ExternalLink,
  Heart,
  Info,
  Layers3,
  Plus,
  Search,
  SlidersHorizontal,
  TriangleAlert,
  Volume2,
  X,
} from "lucide-react";
import { modCount, useLocale } from "../i18n";
import {
  categoryLabel,
  gameGroups,
  installablePackMembers,
  isInstallable,
  modById,
  mods,
  wardrobeGroups,
  type Domain,
  type StudioMod,
} from "./model";
import { categoryHelp } from "./categoryHelp";
import { findPackage, usePackagesRevision } from "./packages";
import { formatBytes } from "./WardrobeReview";
import { Empty, Media, Modal, ModCard, PageHead } from "./ui";
import { CUSTOM_FONTS_CATALOG_ID, setCustomFont, useCustomFont } from "./customFont";
import {
  CUSTOM_BACKGROUNDS_CATALOG_ID,
  loadingScreens,
  setBackgroundChoice,
  useBackgroundChoice,
} from "./customBackground";
import { engineBridge } from "../engine";
import { noticeFor } from "./builder/notices";
import {
  USER_STYLES_CATALOG_ID,
  setUserStylesMask,
  userStyleSnippets,
  useUserStylesMask,
} from "./userStyles";
import { isMapPackage } from "./packages";

export type CatalogFilters = {
  domain: Domain;
  query: string;
  group: string;
  category: string;
  selected: boolean;
  favorites: boolean;
  sort: string;
  limit: number;
};
export const initialFilters: CatalogFilters = {
  domain: "wardrobe",
  query: "",
  group: "all",
  category: "all",
  selected: false,
  favorites: false,
  sort: "featured",
  limit: 24,
};
type Props = {
  filters: CatalogFilters;
  setFilters: Dispatch<SetStateAction<CatalogFilters>>;
  selected: string[];
  favorites: string[];
  onOpen: (mod: StudioMod) => void;
  onToggle: (mod: StudioMod) => void;
};

export default function Catalog({
  filters,
  setFilters,
  selected,
  favorites,
  onOpen,
  onToggle,
}: Props) {
  const { language, isRu } = useLocale();
  const update = (patch: Partial<CatalogFilters>) =>
    setFilters((previous) => ({ ...previous, ...patch, limit: patch.limit ?? 24 }));
  // Only what BetterFy can install is offered. A signed catalog can add
  // packages at runtime, so the list follows the package revision.
  const revision = usePackagesRevision();
  const offered = useMemo(() => mods.filter((mod) => isInstallable(mod.id)), [revision]);
  const source = useMemo(
    () => offered.filter((mod) => mod.domain === filters.domain),
    [offered, filters.domain],
  );
  const groups = filters.domain === "wardrobe" ? wardrobeGroups : gameGroups;
  const categories = useMemo(
    () =>
      [
        ...new Set(
          source
            .filter((mod) => filters.group === "all" || mod.group === filters.group)
            .map((mod) => mod.category),
        ),
      ].sort((a, b) =>
        categoryLabel(a, language).localeCompare(categoryLabel(b, language), language),
      ),
    [source, filters.group, language],
  );
  const results = useMemo(() => {
    const query = filters.query.trim().toLocaleLowerCase();
    return source
      .filter(
        (mod) =>
          (filters.group === "all" || mod.group === filters.group) &&
          (filters.category === "all" || mod.category === filters.category) &&
          (!filters.selected || selected.includes(mod.id)) &&
          (!filters.favorites || favorites.includes(mod.id)) &&
          (!query ||
            `${mod.name.ru} ${mod.name.en} ${mod.categoryName[language]} ${mod.author}`
              .toLocaleLowerCase()
              .includes(query)),
      )
      .sort((a, b) =>
        filters.sort === "name"
          ? a.name[language].localeCompare(b.name[language])
          : filters.sort === "new"
            ? b.date - a.date
            : Number(b.image !== null) - Number(a.image !== null),
      );
  }, [source, filters, selected, favorites, language]);
  const countInDomain = source.filter((mod) => selected.includes(mod.id)).length;
  const activeFilter =
    filters.query ||
    filters.group !== "all" ||
    filters.category !== "all" ||
    filters.selected ||
    filters.favorites;
  return (
    <div className="s-catalog">
      <PageHead
        title={isRu ? "Найди своё." : "Find your next favorite."}
        description={
          isRu
            ? "От одного облика до полностью новой Dota."
            : "From one new look to a completely different Dota."
        }
      >
        <span className="s-catalog-total">{modCount(source.length, language)}</span>
      </PageHead>
      <div className="s-catalog-tools">
        <div
          className="s-domain-tabs"
          role="tablist"
          aria-label={isRu ? "Раздел каталога" : "Catalog section"}
        >
          <button
            role="tab"
            aria-selected={filters.domain === "wardrobe"}
            onClick={() => update({ domain: "wardrobe", group: "all", category: "all" })}
          >
            {isRu ? "Облики и эффекты" : "Looks & effects"}
            <span>{offered.filter((m) => m.domain === "wardrobe").length}</span>
          </button>
          <button
            role="tab"
            aria-selected={filters.domain === "game"}
            onClick={() => update({ domain: "game", group: "all", category: "all" })}
          >
            {isRu ? "Настройка игры" : "Game tuning"}
            <span>{offered.filter((m) => m.domain === "game").length}</span>
          </button>
        </div>
        <div className="s-search-row">
          <label className="s-search">
            <Search />
            <input
              id="studio-catalog-search"
              value={filters.query}
              onChange={(event) => update({ query: event.target.value })}
              placeholder={isRu ? "Название, герой или автор" : "Search a mod, hero or creator"}
              aria-label={isRu ? "Поиск модов" : "Search mods"}
            />
            {filters.query && (
              <button
                className="s-icon"
                aria-label={isRu ? "Очистить поиск" : "Clear search"}
                onClick={() => update({ query: "" })}
              >
                <X />
              </button>
            )}
          </label>
          <button
            className={`s-filter-button ${filters.favorites ? "active" : ""}`}
            aria-pressed={filters.favorites}
            onClick={() => update({ favorites: !filters.favorites })}
          >
            <Heart />
            {isRu ? "Избранное" : "Favorites"}
          </button>
          <button
            className={`s-filter-button ${filters.selected ? "active" : ""}`}
            aria-pressed={filters.selected}
            onClick={() => update({ selected: !filters.selected })}
          >
            <CheckCheck />
            {isRu ? "В сборке" : "Selected"}
            <span>{countInDomain}</span>
          </button>
        </div>
        <div className="s-category-row">
          <div
            className="s-category-tabs"
            role="group"
            aria-label={isRu ? "Категория" : "Category"}
          >
            {groups.map((group) => (
              <button
                key={group.id}
                aria-pressed={filters.group === group.id}
                onClick={() => update({ group: group.id, category: "all" })}
              >
                {group[language]}
              </button>
            ))}
          </div>
          {filters.domain === "wardrobe" && (
            <label className="s-select">
              <SlidersHorizontal />
              <select
                aria-label={isRu ? "Тип предмета" : "Item type"}
                value={filters.category}
                onChange={(event) => update({ category: event.target.value })}
              >
                <option value="all">{isRu ? "Все типы" : "All types"}</option>
                {categories.map((category) => (
                  <option key={category} value={category}>
                    {categoryLabel(category, language)}
                  </option>
                ))}
              </select>
              <ChevronDown />
            </label>
          )}
        </div>
      </div>
      <div className="s-results-head">
        <span>
          {modCount(results.length, language)}
          {filters.query && <> · «{filters.query}»</>}
        </span>
        {activeFilter && (
          <button
            className="s-text-button"
            onClick={() => setFilters({ ...initialFilters, domain: filters.domain })}
          >
            {isRu ? "Сбросить фильтры" : "Reset filters"}
            <X />
          </button>
        )}
        <label className="s-sort">
          <select
            aria-label={isRu ? "Сортировка" : "Sort order"}
            value={filters.sort}
            onChange={(event) => update({ sort: event.target.value })}
          >
            <option value="featured">{isRu ? "Рекомендуемые" : "Featured"}</option>
            <option value="new">{isRu ? "Сначала новые" : "Newest first"}</option>
            <option value="name">{isRu ? "По названию" : "Name A–Z"}</option>
          </select>
          <ChevronDown />
        </label>
      </div>
      {results.length ? (
        <>
          <div className={`s-mod-grid ${filters.domain === "game" ? "is-game" : ""}`}>
            {results.slice(0, filters.limit).map((mod) => (
              <ModCard
                key={mod.id}
                mod={mod}
                selected={
                  mod.members
                    ? installablePackMembers(mod.id).every((id) => selected.includes(id))
                    : selected.includes(mod.id)
                }
                onOpen={() => onOpen(mod)}
                onToggle={() => onToggle(mod)}
              />
            ))}
          </div>
          {results.length > filters.limit && (
            <button
              className="s-load-more s-btn"
              onClick={() => update({ limit: filters.limit + 24 })}
            >
              {isRu ? "Показать ещё" : "Show more"}
              <span>{Math.min(24, results.length - filters.limit)}</span>
              <ArrowRight />
            </button>
          )}
        </>
      ) : (
        <Empty
          icon={<Search />}
          title={isRu ? "Здесь пока ничего нет" : "Nothing here yet"}
          text={
            filters.favorites
              ? isRu
                ? "Отмечай понравившиеся моды сердечком в карточке."
                : "Save mods with the heart in their detail panel."
              : isRu
                ? "Попробуй другое название или убери часть фильтров."
                : "Try another name or remove some filters."
          }
          action={isRu ? "Сбросить фильтры" : "Reset filters"}
          onAction={() => setFilters({ ...initialFilters, domain: filters.domain })}
        />
      )}
    </div>
  );
}

export function ModDetails({
  mod,
  selected,
  favorite,
  variant,
  onVariant,
  onClose,
  onToggle,
  onFavorite,
}: {
  mod: StudioMod;
  selected: boolean;
  favorite: boolean;
  variant: number;
  onVariant: (variant: number) => void;
  onClose: () => void;
  onToggle: () => void;
  onFavorite: () => void;
}) {
  const { language, isRu } = useLocale();
  const [infoOpen, setInfoOpen] = useState(false);
  const preview = mod.variants[variant]?.image ?? mod.image;
  const audited = mod.domain === "wardrobe" ? findPackage(mod.id) : undefined;
  const help = mod.domain === "wardrobe" ? categoryHelp(mod.category) : null;
  return (
    <Modal
      className="s-detail-dialog"
      title={isRu ? "Подробнее о моде" : "Mod details"}
      onClose={onClose}
    >
      <div className="s-detail-media">
        <Media src={preview ?? undefined} showPending pendingLabel={mod.categoryName[language]} />
        <span className="s-detail-category">{mod.categoryName[language]}</span>
      </div>
      <div className="s-detail-content">
        <div className="s-detail-title">
          <div>
            <span className="s-eyebrow">{mod.author}</span>
            <h2>{mod.name[language]}</h2>
          </div>
          <button
            className={`s-icon s-favorite ${favorite ? "active" : ""}`}
            aria-label={isRu ? "В избранное" : "Save to favorites"}
            aria-pressed={favorite}
            onClick={onFavorite}
          >
            <Heart />
          </button>
        </div>
        <p className="s-detail-description">{mod.description[language].split("\n\n")[0]}</p>
        {mod.id === USER_STYLES_CATALOG_ID && <UserStylesPicker />}
        {mod.id === CUSTOM_FONTS_CATALOG_ID && <CustomFontPicker />}
        {mod.id === CUSTOM_BACKGROUNDS_CATALOG_ID && <BackgroundPicker />}
        {mod.members && <PackContents mod={mod} />}
        {mod.variants.length > 1 && (
          <fieldset className="s-variants">
            <legend>{isRu ? "Вариант" : "Style"}</legend>
            {mod.variants.map((style, index) => (
              <button
                key={`${style.name}-${index}`}
                aria-pressed={variant === index}
                onClick={() => onVariant(index)}
              >
                <i style={{ background: style.color }} />
                {style.name}
                {variant === index && <Check />}
              </button>
            ))}
          </fieldset>
        )}
        {mod.domain === "wardrobe" && mod.group === "audio" && (
          <div className="s-inline-note">
            <Volume2 />
            <p>
              {isRu
                ? "Послушай оригинал перед выбором. Изображение не передаёт звучание."
                : "Listen to the original before choosing. The image cannot preview audio."}
            </p>
            <a href={mod.source} target="_blank" rel="noreferrer">
              <ExternalLink />
            </a>
          </div>
        )}
        {audited && help && (
          <div className="s-inline-note">
            <Info />
            <div>
              <p>{help.installs[language]}</p>
              {help.tip && <p>{help.tip[language]}</p>}
            </div>
          </div>
        )}
        {isMapPackage(mod.id) && (
          <div className="s-inline-note warning">
            <TriangleAlert />
            <p>
              {isRu
                ? "Это целая карта Dota, а не облик. Видна только в матчах (не в «Демо»), а террейн Dota+ нужно отключить в настройках игры. После крупного обновления Dota карту может понадобиться переустановить."
                : "This is a whole Dota map, not a skin. It shows only in matches (not in Demo), and the Dota+ terrain has to be turned off in the game settings. After a major Dota update the map may need to be installed again."}
            </p>
          </div>
        )}
        <div className="s-detail-facts">
          <div>
            <span>{isRu ? "Раздел" : "Section"}</span>
            <strong>
              {mod.domain === "game"
                ? isRu
                  ? "Настройка игры"
                  : "Game tuning"
                : isRu
                  ? "Облики и эффекты"
                  : "Looks & effects"}
            </strong>
          </div>
          <div>
            <span>{isRu ? "Автор / источник" : "Creator / source"}</span>
            <strong>{mod.author}</strong>
          </div>
          {mod.domain === "wardrobe" && (
            <div>
              <span>{isRu ? "Автор ресурса" : "Resource author"}</span>
              <a
                className="s-detail-credit"
                href="https://h6rd.github.io/Dota2PornFxWeb/"
                target="_blank"
                rel="noreferrer"
              >
                Dota2PornFxWeb
                <ExternalLink />
              </a>
            </div>
          )}
          {audited?.downloadBytes !== undefined && audited.installBytes !== undefined && (
            <div>
              <span>{isRu ? "Загрузка / в сборке" : "Download / in the build"}</span>
              <strong>
                {formatBytes(audited.downloadBytes, isRu)} /{" "}
                {formatBytes(audited.installBytes, isRu)}
              </strong>
            </div>
          )}
        </div>
        <button
          className="s-disclosure"
          aria-expanded={infoOpen}
          onClick={() => setInfoOpen(!infoOpen)}
        >
          {isRu ? "Источник" : "Source"}
          <ChevronDown className={infoOpen ? "rotated" : ""} />
        </button>
        {infoOpen && (
          <div className="s-disclosed">
            <p>
              {isRu
                ? "Автор и превью — из исходного каталога. Спасибо создателям за их работу."
                : "Creator and preview come from the source catalog. Credit goes to the people who made it."}
            </p>
            <a href={mod.source} target="_blank" rel="noreferrer">
              {isRu ? "Открыть источник" : "Open source"}
              <ExternalLink />
            </a>
          </div>
        )}
      </div>
      <footer className="s-detail-footer">
        <button
          className={`s-btn ${selected ? "s-btn-selected" : "s-btn-primary"}`}
          onClick={onToggle}
        >
          {selected ? <Check /> : <Plus />}
          {selected
            ? isRu
              ? "Убрать из сборки"
              : "Remove from build"
            : isRu
              ? "Добавить в сборку"
              : "Add to build"}
        </button>
        <small>
          <Layers3 />
          {isRu ? "Поставить в Dota — в «Моей сборке»" : "Install it into Dota from My build"}
        </small>
      </footer>
    </Modal>
  );
}

/** The snippets of Minify's User Styles, each with its own switch. */
function UserStylesPicker() {
  const { language, isRu } = useLocale();
  const mask = useUserStylesMask();
  const count = userStyleSnippets.filter((_, index) => mask & (1 << index)).length;
  return (
    <fieldset className="s-style-picker">
      <legend>
        {isRu ? "Что включить" : "What to turn on"}
        <span>
          {count} / {userStyleSnippets.length}
        </span>
      </legend>
      {userStyleSnippets.map((label, index) => {
        const on = Boolean(mask & (1 << index));
        return (
          <label key={label.en} className={on ? "is-on" : ""}>
            <input
              type="checkbox"
              checked={on}
              onChange={() => setUserStylesMask(on ? mask & ~(1 << index) : mask | (1 << index))}
            />
            <span>{label[language]}</span>
          </label>
        );
      })}
      <small>
        {isRu
          ? "Выбор сохраняется и попадает в сборку. Поменял — подготовь сборку заново."
          : "Your pick is saved and goes into the build. After a change, prepare the build again."}
      </small>
    </fieldset>
  );
}

/** Minify's Custom Fonts: the player's own font file, checked by the engine. */
function CustomFontPicker() {
  const { isRu } = useLocale();
  const font = useCustomFont();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function pick(file: File | undefined) {
    if (!file) return;
    setBusy(true);
    setError(null);
    try {
      const base64 = await new Promise<string>((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(String(reader.result).replace(/^data:[^,]*,/, ""));
        reader.onerror = () => reject(reader.error);
        reader.readAsDataURL(file);
      });
      setCustomFont(await engineBridge.storeCustomFont(base64));
    } catch (cause) {
      const code =
        typeof cause === "object" && cause && "code" in cause
          ? String((cause as { code: unknown }).code)
          : String(cause);
      setError(noticeFor(code, isRu)?.body ?? code);
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="s-style-picker s-font-picker">
      <strong>{isRu ? "Шрифт" : "Font"}</strong>
      {font ? (
        <p>
          <b>{font.family}</b> · {formatBytes(font.bytes, isRu)}
        </p>
      ) : (
        <p>{isRu ? "Шрифт ещё не выбран." : "No font picked yet."}</p>
      )}
      <label className="s-btn">
        <input
          type="file"
          accept=".ttf,.otf,font/ttf,font/otf"
          disabled={busy}
          onChange={(event) => void pick(event.target.files?.[0])}
        />
        {busy
          ? isRu
            ? "Проверяем…"
            : "Checking…"
          : font
            ? isRu
              ? "Выбрать другой файл"
              : "Choose another file"
            : isRu
              ? "Выбрать файл .ttf или .otf"
              : "Choose a .ttf or .otf file"}
      </label>
      {error && <p className="s-font-error">{error}</p>}
      <small>
        {isRu
          ? "Шрифт заменит Radiance и Reaver во всём интерфейсе Dota. Нужны русские буквы — выбирай шрифт с кириллицей. Шрифты Windows лежат в C:\\Windows\\Fonts."
          : "The font replaces Radiance and Reaver across Dota's interface. Pick a font with the letters your language needs. Windows keeps its fonts in C:\\Windows\\Fonts."}
      </small>
    </div>
  );
}

/** Minify's Custom Backgrounds: a loading screen of the game or a WebM. */
function BackgroundPicker() {
  const { isRu } = useLocale();
  const choice = useBackgroundChoice();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function pickVideo(file: File | undefined) {
    if (!file) return;
    setBusy(true);
    setError(null);
    try {
      const base64 = await new Promise<string>((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(String(reader.result).replace(/^data:[^,]*,/, ""));
        reader.onerror = () => reject(reader.error);
        reader.readAsDataURL(file);
      });
      const video = await engineBridge.storeCustomVideo(base64);
      setBackgroundChoice({ kind: "video", packageId: video.packageId, bytes: video.bytes });
    } catch (cause) {
      const code =
        typeof cause === "object" && cause && "code" in cause
          ? String((cause as { code: unknown }).code)
          : String(cause);
      setError(noticeFor(code, isRu)?.body ?? code);
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="s-style-picker s-font-picker">
      <strong>{isRu ? "Фон меню" : "Menu background"}</strong>
      <label className="s-select s-background-select">
        <select
          aria-label={isRu ? "Экран загрузки" : "Loading screen"}
          value={choice.kind === "screen" ? choice.id : "video"}
          onChange={(event) => {
            if (event.target.value !== "video")
              setBackgroundChoice({ kind: "screen", id: event.target.value });
          }}
        >
          {loadingScreens.map((screen) => (
            <option key={screen.id} value={screen.id}>
              {screen.label}
            </option>
          ))}
          {choice.kind === "video" && (
            <option value="video">
              {isRu ? "Своё видео" : "Your video"} · {formatBytes(choice.bytes, isRu)}
            </option>
          )}
        </select>
        <ChevronDown />
      </label>
      <label className="s-btn">
        <input
          type="file"
          accept=".webm,video/webm"
          disabled={busy}
          onChange={(event) => void pickVideo(event.target.files?.[0])}
        />
        {busy
          ? isRu
            ? "Проверяем видео…"
            : "Checking the video…"
          : isRu
            ? "Своё видео (.webm)"
            : "Your own video (.webm)"}
      </label>
      {error && <p className="s-font-error">{error}</p>}
      <small>
        {isRu
          ? "Экран загрузки берётся из самой Dota. Видео — файл WebM до 64 МБ; другие форматы можно перевести в WebM любым конвертером."
          : "The loading screen comes from Dota itself. A video has to be a WebM file up to 64 MB; other formats convert to WebM with any converter."}
      </small>
    </div>
  );
}

/** What a pack adds: its items that install, with the ones already in the build. */
function PackContents({ mod }: { mod: StudioMod }) {
  const { language, isRu } = useLocale();
  const members = installablePackMembers(mod.id);
  return (
    <div className="s-style-picker s-pack-contents">
      <strong>
        {isRu ? "Что внутри" : "Inside"} · {members.length}
      </strong>
      <ul>
        {members.map((id) => {
          const item = modById.get(id);
          return item ? (
            <li key={id}>
              <span>{item.name[language]}</span>
              <small>{item.categoryName[language]}</small>
            </li>
          ) : null;
        })}
      </ul>
    </div>
  );
}
