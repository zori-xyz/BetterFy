import { useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowRight,
  Check,
  ExternalLink,
  ImageOff,
  PackageOpen,
  Plus,
  Search,
  ShieldQuestion,
  Shirt,
  SlidersHorizontal,
  Sparkles,
  Wrench,
  X,
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
    eyebrow: "ГАРДЕРОБ",
    title: "Собери внешний вид Dota",
    text: "Герои, предметы, эффекты, HUD и звук. Здесь нет оптимизации и игровых утилит.",
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
    buildEmpty: "Выбери первый мод. Он появится здесь и сохранится между разделами.",
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
    eyebrow: "WARDROBE",
    title: "Build your Dota look",
    text: "Heroes, items, effects, HUDs, and audio. Performance mods and game utilities stay out of this space.",
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

const wardrobeSearchAliases: Record<string, Record<Language, string[]>> = {
  backgrounds: { ru: ["фон", "фоны", "задник"], en: ["background", "backgrounds", "backdrop"] },
  creeps: { ru: ["крип", "крипы", "существа"], en: ["creep", "creeps", "units"] },
  "creep-deny": { ru: ["добивание", "денай"], en: ["deny", "creep deny"] },
  towers: { ru: ["башня", "башни", "вышки"], en: ["tower", "towers"] },
  wards: { ru: ["вард", "варды", "тотем"], en: ["ward", "wards", "totem"] },
  couriers: { ru: ["курьер", "курьеры"], en: ["courier", "couriers"] },
  heroes: { ru: ["герой", "герои", "персонаж"], en: ["hero", "heroes", "character"] },
  "hero-items": { ru: ["предмет", "предметы", "шмотка", "скин"], en: ["item", "items", "skin"] },
  hud: { ru: ["интерфейс", "худ", "панель"], en: ["interface", "hud"] },
  sounds: { ru: ["звук", "звуки", "озвучка"], en: ["sound", "sounds", "audio"] },
  music: { ru: ["музыка", "саундтрек"], en: ["music", "soundtrack"] },
  effects: { ru: ["эффект", "эффекты", "анимация"], en: ["effect", "effects", "animation"] },
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
  const [activeId, setActiveId] = useState(
    wardrobeCatalogItems.find((mod) => mod.metadata.name === "Queen of Pain Rose")?.id
      ?? wardrobeCatalogItems[0]?.id
      ?? "",
  );
  const [failedImages, setFailedImages] = useState<Set<string>>(() => new Set());
  const [detailOpen, setDetailOpen] = useState(false);
  const featureRef = useRef<HTMLElement>(null);

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
        ...(wardrobeSearchAliases[mod.metadata.category]?.[language] ?? []),
      ]
        .filter(Boolean)
        .some((value) => value!.toLocaleLowerCase(language).includes(normalized));
    }).sort((a, b) => sort === "name"
      ? a.metadata.name.localeCompare(b.metadata.name, language)
      : (b.verification.updatedAt ?? 0) - (a.verification.updatedAt ?? 0));
  }, [category, group, groupItems, language, query, selectedIds, selectedOnly, sort]);

  const updateQuery = (value: string) => {
    setQuery(value);
    if (value.trim()) {
      const normalized = value.trim().toLocaleLowerCase(language);
      const matchedCategory = Object.keys(wardrobeCategoryLabels).find((key) => {
        const label = wardrobeCategoryLabels[key]?.[language].toLocaleLowerCase(language);
        const aliases = wardrobeSearchAliases[key]?.[language] ?? [];
        return label === normalized || aliases.some((alias) => alias === normalized);
      });
      setGroup("all");
      setCategory(matchedCategory ?? "all");
    }
  };

  useEffect(() => {
    if (!filtered.some((mod) => mod.id === activeId)) setActiveId(filtered[0]?.id ?? "");
  }, [activeId, filtered]);

  useEffect(() => setLimit(12), [category, group, query, selectedOnly, sort]);

  const active = filtered.find((mod) => mod.id === activeId) ?? filtered[0] ?? null;
  const markFailed = (id: string) =>
    setFailedImages((current) => {
      const next = new Set(current);
      next.add(id);
      return next;
    });

  const openMod = (id: string) => {
    setActiveId(id);
    setDetailOpen(true);
    window.requestAnimationFrame(() => featureRef.current?.focus());
  };

  useEffect(() => {
    if (!detailOpen) return undefined;
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setDetailOpen(false);
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [detailOpen]);

  return (
    <div className="mod-catalog-route">
      <header className="catalog-heading">
        <div>
          <span><Sparkles />{t.eyebrow}</span>
          <h1 className="accent-title"><AccentTitle text={t.title} /></h1>
          <p>{t.text}</p>
        </div>
      </header>

      <div className="catalog-controls">
        <label>
          <Search />
          <input value={query} onChange={(event) => updateQuery(event.target.value)} placeholder={t.search} />
          {query && <button onClick={() => setQuery("")} aria-label={language === "ru" ? "Очистить поиск" : "Clear search"}>×</button>}
        </label>
        <div className="catalog-groups" role="tablist" aria-label={language === "ru" ? "Категории модов" : "Mod categories"}>
          {(Object.keys(modGroupLabels) as ModCatalogGroup[]).filter((key) => key === "all" || groupCounts[key] > 0).map((key) => (
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
          {detailOpen && <div className="catalog-detail-layer" onMouseDown={() => setDetailOpen(false)}>
          <section
            ref={featureRef}
            className="catalog-feature catalog-detail-dialog"
            key={active.id}
            role="dialog"
            aria-modal="true"
            aria-label={active.metadata.name}
            tabIndex={-1}
            onMouseDown={(event) => event.stopPropagation()}
          >
            <button className="catalog-detail-close" aria-label={language === "ru" ? "Закрыть подробности" : "Close details"} onClick={() => setDetailOpen(false)}><X /></button>
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
          </div>}

          <section className="catalog-results">
            <header>
              <div><strong>{filtered.length}</strong><span>{t.results}</span></div>
              <small>{language === "ru" ? "Нажми карточку для подробностей или добавь мод сразу" : "Open a card for details or add the mod immediately"}</small>
            </header>
            <div className="catalog-grid">
              {filtered.slice(0, limit).map((mod) => {
                const selected = selectedIds.includes(mod.id);
                return (
                  <article className={`${mod.id === active.id ? "active" : ""} ${selected ? "selected" : ""}`} key={mod.id}>
                    <button className="catalog-card-preview" onClick={() => openMod(mod.id)}>
                      <CatalogPreview
                        mod={mod}
                        language={language}
                        failed={failedImages.has(mod.id)}
                        onFailure={() => markFailed(mod.id)}
                      />
                      <span>{mod.metadata.categoryLabel[language]}</span>
                    </button>
                    <button className="catalog-card-copy" aria-label={`${t.details}: ${mod.metadata.name}`} onClick={() => openMod(mod.id)}>
                      <strong>{mod.metadata.name}</strong>
                      <p>{mod.presentation.description[language]}</p>
                      <small>
                        <span>{mod.provenance.author ?? mod.provenance.sourceName}</span>
                        <b>{mod.metadata.styleCount > 1 ? `${mod.metadata.styleCount} ${t.styles}` : mod.artifact.fileKind}</b>
                      </small>
                      <span className="catalog-card-open">{t.details}<ArrowRight /></span>
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
  selectedWardrobeIds,
  selectedGameIds,
  onToggleWardrobe,
  onToggleGame,
}: {
  language: Language;
  selectedWardrobeIds: string[];
  selectedGameIds: string[];
  onToggleWardrobe: (id: string) => void;
  onToggleGame: (id: string) => void;
}) {
  const [section, setSection] = useState<"mods" | "wardrobe">("mods");
  const hubRef = useRef<HTMLDivElement>(null);
  const scrollPositions = useRef({ mods: 0, wardrobe: 0 });
  const labels = language === "ru"
    ? {
        eyebrow: "BETTERFY / LOADOUT STUDIO",
        title: "Собери свою Dota",
        text: "Сначала выбери, что меняется. Затем открой мод в сцене, добавь его и проверь общую сборку.",
        steps: ["Выбери раздел", "Открой мод", "Добавь", "Проверь сборку"],
        mods: "Игра",
        modsKind: "ПРОИЗВОДИТЕЛЬНОСТЬ И УТИЛИТЫ",
        modsHint: `FPS, карта, HUD и звук · ${minifyMods.length}`,
        wardrobe: "Гардероб",
        wardrobeKind: "ВНЕШНИЙ ВИД",
        wardrobeHint: `Герои, предметы и эффекты · ${wardrobeCatalogItems.length}`,
        open: "Открыто",
      }
    : {
        eyebrow: "BETTERFY / LOADOUT STUDIO",
        title: "Build your Dota",
        text: "Choose what changes, open a mod in the scene, add it, then review the complete build.",
        steps: ["Choose a space", "Open a mod", "Add it", "Review build"],
        mods: "Game",
        modsKind: "PERFORMANCE AND UTILITIES",
        modsHint: `FPS, map, HUD, and audio · ${minifyMods.length}`,
        wardrobe: "Wardrobe",
        wardrobeKind: "APPEARANCE",
        wardrobeHint: `Heroes, items, and effects · ${wardrobeCatalogItems.length}`,
        open: "Open",
      };

  const switchSection = (next: "mods" | "wardrobe") => {
    if (next === section) return;
    const hub = hubRef.current;
    if (hub) scrollPositions.current[section] = hub.scrollTop;
    setSection(next);
    window.requestAnimationFrame(() => {
      if (hubRef.current) hubRef.current.scrollTop = scrollPositions.current[next];
    });
  };

  return (
    <div ref={hubRef} className="catalog-hub">
      <nav className="catalog-section-switch" data-section={section} aria-label={language === "ru" ? "Раздел каталога" : "Catalog section"}>
        <div className="catalog-domain-question">
          <span>{labels.eyebrow}</span>
          <strong>{labels.title}</strong>
          <p>{labels.text}</p>
        </div>
        <div className="catalog-route-map" aria-label={language === "ru" ? "Порядок работы" : "Catalog path"}>
          {labels.steps.map((step, index) => <span className={index === 0 ? "active" : ""} key={step}><b>0{index + 1}</b>{step}</span>)}
        </div>
        <div className="catalog-mode-picker">
        <button
          className={section === "mods" ? "active" : ""}
          data-domain="game"
          aria-current={section === "mods" ? "page" : undefined}
          onClick={() => switchSection("mods")}
        >
          <span className="catalog-domain-icon"><Wrench /></span>
          <span className="catalog-domain-copy">
            <small>{labels.modsKind}</small>
            <strong>{labels.mods}</strong>
            <em>{labels.modsHint}</em>
          </span>
          <span className="catalog-domain-state">{section === "mods" ? <><Check />{labels.open}</> : <ArrowRight />}</span>
        </button>
        <button
          className={section === "wardrobe" ? "active" : ""}
          data-domain="wardrobe"
          aria-current={section === "wardrobe" ? "page" : undefined}
          onClick={() => switchSection("wardrobe")}
        >
          <span className="catalog-domain-icon"><Shirt /></span>
          <span className="catalog-domain-copy">
            <small>{labels.wardrobeKind}</small>
            <strong>{labels.wardrobe}</strong>
            <em>{labels.wardrobeHint}</em>
          </span>
          <span className="catalog-domain-state">{section === "wardrobe" ? <><Check />{labels.open}</> : <ArrowRight />}</span>
        </button>
        </div>
      </nav>
      <div hidden={section !== "mods"}>
        <MinifyModsRoute language={language} selectedIds={selectedGameIds} onToggle={onToggleGame} />
      </div>
      <div hidden={section !== "wardrobe"}>
        <WardrobeCatalogRoute language={language} selectedIds={selectedWardrobeIds} onToggle={onToggleWardrobe} />
      </div>
    </div>
  );
}
