import { useEffect, useMemo, useRef, useState, type UIEvent } from "react";
import {
  ArrowRight,
  Check,
  ExternalLink,
  ImageOff,
  Layers3,
  PackageOpen,
  Plus,
  Search,
  ShieldQuestion,
  Shirt,
  SlidersHorizontal,
  Sparkles,
  Wrench,
} from "lucide-react";
import type { Language } from "./i18n";
import AccentTitle from "./AccentTitle";
import {
  modGroupLabels,
  wardrobeCategoryLabels,
  wardrobeCatalogItems,
  type BetterFyCatalogMod,
  type ModCatalogGroup,
} from "./modCatalog";
import { minifyMods } from "./minifyCatalog";
import MinifyModsRoute from "./MinifyModsRoute";

const copy = {
  ru: {
    eyebrow: "ГАРДЕРОБ / DOTA2PORNFX",
    title: "Визуальная коллекция для твоей Dota",
    text: "Скины, HUD, эффекты, звук и оформление из Dota2PornFxWeb. Этот раздел отделён от функциональных модов Minify.",
    search: "Найти мод, автора или категорию",
    source: "Источник",
    author: "Автор",
    unknownAuthor: "Автор не указан",
    compatibility: "Совместимость",
    unknown: "Не проверена",
    permission: "Права",
    permissionUnknown: "Требуют проверки",
    archive: "Архив",
    styles: "варианта",
    add: "Добавить в сборку",
    remove: "Убрать из сборки",
    selected: "В сборке",
    build: "ТВОЯ СБОРКА",
    buildEmpty: "Выбери первый мод — он появится здесь и сохранится между разделами.",
    buildHint: "Патчинг пока не выполняется",
    results: "модов",
    showMore: "Показать ещё",
    empty: "Ничего не найдено",
    emptyText: "Попробуй другой запрос или вернись ко всем категориям.",
    previewMissing: "Превью недоступно",
    sourceSnapshot: "Снимок каталога",
    originNote: "Названия и превью сохранены из источника. Описания адаптированы BetterFy.",
    selectedOnly: "Только выбранные",
    allTypes: "Все типы",
    sortRecent: "Сначала новые",
    sortName: "По названию",
    reset: "Сбросить фильтры",
    details: "Открыть детали",
  },
  en: {
    eyebrow: "WARDROBE / DOTA2PORNFX",
    title: "A visual collection for your Dota",
    text: "Skins, HUDs, effects, audio, and presentation from Dota2PornFxWeb. This section is separate from functional Minify mods.",
    search: "Find a mod, author, or category",
    source: "Source",
    author: "Author",
    unknownAuthor: "Author not listed",
    compatibility: "Compatibility",
    unknown: "Not verified",
    permission: "Rights",
    permissionUnknown: "Needs review",
    archive: "Archive",
    styles: "styles",
    add: "Add to build",
    remove: "Remove from build",
    selected: "In build",
    build: "YOUR BUILD",
    buildEmpty: "Choose the first mod. It will stay here as you move through the app.",
    buildHint: "Patching is not active yet",
    results: "mods",
    showMore: "Show more",
    empty: "Nothing found",
    emptyText: "Try another query or return to all categories.",
    previewMissing: "Preview unavailable",
    sourceSnapshot: "Catalog snapshot",
    originNote: "Names and previews come from the source. BetterFy adapts the descriptions.",
    selectedOnly: "Selected only",
    allTypes: "All types",
    sortRecent: "Newest first",
    sortName: "By name",
    reset: "Reset filters",
    details: "Open details",
  },
};

function CatalogPreview({
  mod,
  failed,
  onFailure,
  language,
  eager = false,
}: {
  mod: BetterFyCatalogMod;
  failed: boolean;
  onFailure: () => void;
  language: Language;
  eager?: boolean;
}) {
  const t = copy[language];
  if (!mod.presentation.previewUrl || failed) {
    return (
      <div className="catalog-image-fallback" aria-label={t.previewMissing}>
        <ImageOff />
        <span>{mod.metadata.name.slice(0, 2).toUpperCase()}</span>
      </div>
    );
  }
  return (
    <img
      src={mod.presentation.previewUrl}
      alt=""
      loading={eager ? "eager" : "lazy"}
      decoding="async"
      referrerPolicy="no-referrer"
      onError={onFailure}
    />
  );
}

function WardrobeCatalogRoute({
  language,
  selectedIds,
  onToggle,
}: {
  language: Language;
  selectedIds: string[];
  onToggle: (id: string) => void;
}) {
  const t = copy[language];
  const [group, setGroup] = useState<ModCatalogGroup>("all");
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState("all");
  const [selectedOnly, setSelectedOnly] = useState(false);
  const [sort, setSort] = useState<"recent" | "name">("recent");
  const [limit, setLimit] = useState(12);
  const [activeId, setActiveId] = useState(wardrobeCatalogItems[0]?.id ?? "");
  const [failedImages, setFailedImages] = useState<Set<string>>(() => new Set());

  const groupItems = useMemo(
    () => wardrobeCatalogItems.filter((mod) => group === "all" || mod.metadata.group === group),
    [group],
  );
  const categoryCounts = useMemo(
    () => groupItems.reduce<Record<string, number>>((counts, mod) => {
      counts[mod.metadata.category] = (counts[mod.metadata.category] ?? 0) + 1;
      return counts;
    }, {}),
    [groupItems],
  );
  const availableCategories = useMemo(
    () => Object.keys(categoryCounts).sort((a, b) =>
      (wardrobeCategoryLabels[a]?.[language] ?? a).localeCompare(wardrobeCategoryLabels[b]?.[language] ?? b, language)),
    [categoryCounts, language],
  );
  const groupCounts = useMemo(
    () => wardrobeCatalogItems.reduce<Record<ModCatalogGroup, number>>(
      (counts, mod) => {
        counts.all += 1;
        counts[mod.metadata.group] += 1;
        return counts;
      },
      { all: 0, characters: 0, interface: 0, audio: 0, world: 0, effects: 0, extras: 0 },
    ),
    [],
  );

  const filtered = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase(language);
    return groupItems.filter((mod) => {
      if (category !== "all" && mod.metadata.category !== category) return false;
      if (selectedOnly && !selectedIds.includes(mod.id)) return false;
      if (group !== "all" && mod.metadata.group !== group) return false;
      if (!normalized) return true;
      return [
        mod.metadata.name,
        mod.provenance.author,
        mod.metadata.categoryLabel[language],
        mod.metadata.category,
      ]
        .filter(Boolean)
        .some((value) => value!.toLocaleLowerCase(language).includes(normalized));
    }).sort((a, b) => sort === "name"
      ? a.metadata.name.localeCompare(b.metadata.name, language)
      : (b.verification.updatedAt ?? 0) - (a.verification.updatedAt ?? 0));
  }, [category, group, groupItems, language, query, selectedIds, selectedOnly, sort]);

  useEffect(() => {
    if (!filtered.some((mod) => mod.id === activeId)) setActiveId(filtered[0]?.id ?? "");
  }, [activeId, filtered]);

  useEffect(() => setLimit(12), [category, group, query, selectedOnly, sort]);

  const active = filtered.find((mod) => mod.id === activeId) ?? filtered[0] ?? null;
  const selectedMods = selectedIds
    .map((id) => wardrobeCatalogItems.find((mod) => mod.id === id))
    .filter((mod): mod is BetterFyCatalogMod => Boolean(mod));

  const markFailed = (id: string) =>
    setFailedImages((current) => {
      const next = new Set(current);
      next.add(id);
      return next;
    });

  return (
    <div className="mod-catalog-route">
      <header className="catalog-heading">
        <div>
          <span><Sparkles />{t.eyebrow}</span>
          <h1 className="accent-title"><AccentTitle text={t.title} /></h1>
          <p>{t.text}</p>
        </div>
        <aside className={`catalog-build-summary ${selectedMods.length ? "has-items" : ""}`}>
          <span>{t.build}</span>
          <strong>{String(selectedMods.length).padStart(2, "0")}</strong>
          <p>{selectedMods.length ? selectedMods.slice(-2).map((mod) => mod.metadata.name).join(" · ") : t.buildEmpty}</p>
          <small><Layers3 />{t.buildHint}</small>
        </aside>
      </header>

      <div className="catalog-controls">
        <label>
          <Search />
          <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t.search} />
          {query && <button onClick={() => setQuery("")} aria-label={language === "ru" ? "Очистить поиск" : "Clear search"}>×</button>}
        </label>
        <div className="catalog-groups" role="tablist" aria-label={language === "ru" ? "Категории модов" : "Mod categories"}>
          {(Object.keys(modGroupLabels) as ModCatalogGroup[]).map((key) => (
            <button
              role="tab"
              aria-selected={group === key}
              className={group === key ? "active" : ""}
              key={key}
              onClick={() => { setGroup(key); setCategory("all"); }}
            >
              <span>{modGroupLabels[key][language]}</span><small>{groupCounts[key]}</small>
            </button>
          ))}
        </div>
        <div className="catalog-fine-controls">
          <div className="catalog-fine-categories" aria-label={language === "ru" ? "Тип предмета" : "Item type"}>
            <button className={category === "all" ? "active" : ""} onClick={() => setCategory("all")}>
              {t.allTypes}<small>{groupItems.length}</small>
            </button>
            {availableCategories.map((key) => (
              <button className={category === key ? "active" : ""} key={key} onClick={() => setCategory(key)}>
                {wardrobeCategoryLabels[key]?.[language] ?? key}<small>{categoryCounts[key]}</small>
              </button>
            ))}
          </div>
          <div className="catalog-filter-actions">
            <button
              className={selectedOnly ? "active" : ""}
              aria-pressed={selectedOnly}
              disabled={!selectedIds.length}
              onClick={() => setSelectedOnly((current) => !current)}
            >
              <Check />{t.selectedOnly}<small>{selectedIds.length}</small>
            </button>
            <label><SlidersHorizontal /><select value={sort} onChange={(event) => setSort(event.target.value as "recent" | "name")}>
              <option value="recent">{t.sortRecent}</option>
              <option value="name">{t.sortName}</option>
            </select></label>
          </div>
        </div>
      </div>

      {active ? (
        <>
          <section className="catalog-feature" key={active.id}>
            <div className="catalog-feature-visual">
              <CatalogPreview
                mod={active}
                language={language}
                eager
                failed={failedImages.has(active.id)}
                onFailure={() => markFailed(active.id)}
              />
              <span>{active.metadata.categoryLabel[language]}</span>
              <i className={selectedIds.includes(active.id) ? "selected" : ""}>
                {selectedIds.includes(active.id) ? <Check /> : <Plus />}
                {selectedIds.includes(active.id) ? t.selected : t.add}
              </i>
            </div>
            <div className="catalog-feature-copy">
              <span>{active.provenance.sourceName}</span>
              <h2>{active.metadata.name}</h2>
              <p>{active.presentation.description[language]}</p>
              {active.metadata.styleCount > 1 && <small>{active.metadata.styleCount} {t.styles}</small>}
              <button
                className={selectedIds.includes(active.id) ? "catalog-primary selected" : "catalog-primary"}
                aria-pressed={selectedIds.includes(active.id)}
                onClick={() => onToggle(active.id)}
              >
                <span>{selectedIds.includes(active.id) ? <Check /> : <Plus />}</span>
                <strong>{selectedIds.includes(active.id) ? t.remove : t.add}</strong>
                <ArrowRight />
              </button>
              <dl>
                <div><dt>{t.author}</dt><dd>{active.provenance.author ?? t.unknownAuthor}</dd></div>
                <div><dt>{t.compatibility}</dt><dd><ShieldQuestion />{t.unknown}</dd></div>
                <div><dt>{t.archive}</dt><dd><PackageOpen />{active.artifact.fileKind}</dd></div>
                <div><dt>{t.permission}</dt><dd>{t.permissionUnknown}</dd></div>
              </dl>
              <div className="catalog-source-line">
                <span>{t.sourceSnapshot}: {active.provenance.snapshotDate}</span>
                <a href={active.provenance.sourceUrl} target="_blank" rel="noreferrer">{t.source}<ExternalLink /></a>
              </div>
            </div>
          </section>

          <section className="catalog-results">
            <header>
              <div><strong>{filtered.length}</strong><span>{t.results}</span></div>
              <small>{t.originNote}</small>
            </header>
            <div className="catalog-grid">
              {filtered.slice(0, limit).map((mod) => {
                const selected = selectedIds.includes(mod.id);
                return (
                  <article className={`${mod.id === active.id ? "active" : ""} ${selected ? "selected" : ""}`} key={mod.id}>
                    <button className="catalog-card-preview" onClick={() => setActiveId(mod.id)}>
                      <CatalogPreview
                        mod={mod}
                        language={language}
                        failed={failedImages.has(mod.id)}
                        onFailure={() => markFailed(mod.id)}
                      />
                      <span>{mod.metadata.categoryLabel[language]}</span>
                    </button>
                    <button className="catalog-card-copy" aria-label={`${t.details}: ${mod.metadata.name}`} onClick={() => setActiveId(mod.id)}>
                      <strong>{mod.metadata.name}</strong>
                      <p>{mod.presentation.description[language]}</p>
                      <small>
                        <span>{mod.provenance.author ?? mod.provenance.sourceName}</span>
                        <b>{mod.metadata.styleCount > 1 ? `${mod.metadata.styleCount} ${t.styles}` : mod.artifact.fileKind}</b>
                      </small>
                    </button>
                    <button
                      className="catalog-card-toggle"
                      aria-label={`${selected ? t.remove : t.add}: ${mod.metadata.name}`}
                      aria-pressed={selected}
                      onClick={() => onToggle(mod.id)}
                    >
                      {selected ? <Check /> : <Plus />}
                      <span>{selected ? t.selected : t.add}</span>
                    </button>
                  </article>
                );
              })}
            </div>
            {limit < filtered.length && (
              <button className="catalog-more" onClick={() => setLimit((current) => current + 12)}>
                {t.showMore}<span>{Math.min(12, filtered.length - limit)}</span><ArrowRight />
              </button>
            )}
          </section>
        </>
      ) : (
        <div className="catalog-empty">
          <Search />
          <h2>{t.empty}</h2>
          <p>{t.emptyText}</p>
          <button onClick={() => { setQuery(""); setGroup("all"); setCategory("all"); setSelectedOnly(false); }}>{t.reset}</button>
        </div>
      )}
    </div>
  );
}

export default function ModCatalogRoute({
  language,
  selectedIds,
  onToggle,
  onNavigationCompactChange,
}: {
  language: Language;
  selectedIds: string[];
  onToggle: (id: string) => void;
  onNavigationCompactChange?: (compact: boolean) => void;
}) {
  const [section, setSection] = useState<"mods" | "wardrobe">("mods");
  const [scrolled, setScrolled] = useState(false);
  const frameRequest = useRef<number | null>(null);
  const compactRef = useRef(false);
  const labels = language === "ru"
    ? { mods: "Моды", modsHint: `${minifyMods.length} · игра и оптимизация`, wardrobe: "Гардероб", wardrobeHint: `${wardrobeCatalogItems.length} · скины и оформление` }
    : { mods: "Mods", modsHint: `${minifyMods.length} · game and performance`, wardrobe: "Wardrobe", wardrobeHint: `${wardrobeCatalogItems.length} · skins and presentation` };

  useEffect(() => () => {
    if (frameRequest.current !== null) window.cancelAnimationFrame(frameRequest.current);
    document.documentElement.style.removeProperty("--catalog-scroll-progress");
    onNavigationCompactChange?.(false);
  }, [onNavigationCompactChange]);

  const handleScroll = (event: UIEvent<HTMLDivElement>) => {
    const scrollTop = event.currentTarget.scrollTop;
    if (frameRequest.current !== null) window.cancelAnimationFrame(frameRequest.current);
    frameRequest.current = window.requestAnimationFrame(() => {
      const progress = Math.min(1, Math.max(0, scrollTop / 180));
      const nextCompact = compactRef.current ? scrollTop > 92 : scrollTop >= 174;
      document.documentElement.style.setProperty("--catalog-scroll-progress", progress.toFixed(3));
      setScrolled(scrollTop > 28);
      if (nextCompact !== compactRef.current) {
        compactRef.current = nextCompact;
        onNavigationCompactChange?.(nextCompact);
      }
    });
  };

  return (
    <div className={`catalog-hub ${scrolled ? "is-scrolled" : ""}`} onScroll={handleScroll}>
      <nav className="catalog-section-switch" aria-label={language === "ru" ? "Раздел каталога" : "Catalog section"}>
        <button className={section === "mods" ? "active" : ""} aria-current={section === "mods" ? "page" : undefined} onClick={() => setSection("mods")}>
          <Wrench /><span><strong>{labels.mods}</strong><small>{labels.modsHint}</small></span>
        </button>
        <i aria-hidden="true">/</i>
        <button className={section === "wardrobe" ? "active" : ""} aria-current={section === "wardrobe" ? "page" : undefined} onClick={() => setSection("wardrobe")}>
          <Shirt /><span><strong>{labels.wardrobe}</strong><small>{labels.wardrobeHint}</small></span>
        </button>
      </nav>
      {section === "mods"
        ? <MinifyModsRoute language={language} />
        : <WardrobeCatalogRoute language={language} selectedIds={selectedIds} onToggle={onToggle} />}
    </div>
  );
}
