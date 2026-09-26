import { useMemo, useState, type Dispatch, type SetStateAction } from "react";
import { ArrowLeft, ArrowRight, Check, CheckCheck, ChevronDown, ExternalLink, Heart, Layers3, Plus, Search, SlidersHorizontal, Volume2, X } from "lucide-react";
import { modCount, useLocale } from "../i18n";
import { categoryLabel, deliveryLabel, gameGroups, isPilotMod, mods, wardrobeGroups, type Domain, type StudioMod } from "./model";
import { Empty, Media, Modal, ModCard, PageHead } from "./ui";

export type CatalogFilters = { domain: Domain; query: string; group: string; category: string; selected: boolean; favorites: boolean; sort: string; limit: number };
export const initialFilters: CatalogFilters = { domain: "wardrobe", query: "", group: "all", category: "all", selected: false, favorites: false, sort: "featured", limit: 24 };
type Props = { filters: CatalogFilters; setFilters: Dispatch<SetStateAction<CatalogFilters>>; selected: string[]; favorites: string[]; onOpen: (mod: StudioMod) => void; onToggle: (mod: StudioMod) => void };

export default function Catalog({ filters, setFilters, selected, favorites, onOpen, onToggle }: Props) {
  const { language, isRu } = useLocale();
  const update = (patch: Partial<CatalogFilters>) => setFilters(previous => ({ ...previous, ...patch, limit: patch.limit ?? 24 }));
  const source = useMemo(() => mods.filter(mod => mod.domain === filters.domain), [filters.domain]);
  const groups = filters.domain === "wardrobe" ? wardrobeGroups : gameGroups;
  const categories = useMemo(() => [...new Set(source.filter(mod => filters.group === "all" || mod.group === filters.group).map(mod => mod.category))].sort((a, b) => categoryLabel(a, language).localeCompare(categoryLabel(b, language), language)), [source, filters.group, language]);
  const results = useMemo(() => {
    const query = filters.query.trim().toLocaleLowerCase();
    return source.filter(mod => (filters.group === "all" || mod.group === filters.group) && (filters.category === "all" || mod.category === filters.category) && (!filters.selected || selected.includes(mod.id)) && (!filters.favorites || favorites.includes(mod.id)) && (!query || `${mod.name.ru} ${mod.name.en} ${mod.categoryName[language]} ${mod.author}`.toLocaleLowerCase().includes(query))).sort((a, b) => filters.sort === "name" ? a.name[language].localeCompare(b.name[language]) : filters.sort === "new" ? b.date - a.date : Number(b.image !== null) - Number(a.image !== null));
  }, [source, filters, selected, favorites, language]);
  const countInDomain = source.filter(mod => selected.includes(mod.id)).length;
  const activeFilter = filters.query || filters.group !== "all" || filters.category !== "all" || filters.selected || filters.favorites;
  return <div className="s-catalog">
    <PageHead title={isRu ? "Найди своё." : "Find your next favorite."} description={isRu ? "От одного облика до полностью новой Dota." : "From one new look to a completely different Dota."}>
      <span className="s-catalog-total">{modCount(source.length, language)}</span>
    </PageHead>
    <div className="s-catalog-tools">
      <div className="s-domain-tabs" role="tablist" aria-label={isRu ? "Раздел каталога" : "Catalog section"}>
        <button role="tab" aria-selected={filters.domain === "wardrobe"} onClick={() => update({ domain: "wardrobe", group: "all", category: "all" })}>{isRu ? "Облики и эффекты" : "Looks & effects"}<span>{mods.filter(m => m.domain === "wardrobe").length}</span></button>
        <button role="tab" aria-selected={filters.domain === "game"} onClick={() => update({ domain: "game", group: "all", category: "all" })}>{isRu ? "Настройка игры" : "Game tuning"}<span>{mods.filter(m => m.domain === "game").length}</span></button>
      </div>
      <div className="s-search-row"><label className="s-search"><Search /><input id="studio-catalog-search" value={filters.query} onChange={event => update({ query: event.target.value })} placeholder={isRu ? "Название, герой или автор" : "Search a mod, hero or creator"} aria-label={isRu ? "Поиск модов" : "Search mods"} />{filters.query && <button className="s-icon" aria-label={isRu ? "Очистить поиск" : "Clear search"} onClick={() => update({ query: "" })}><X /></button>}</label>
        <button className={`s-filter-button ${filters.favorites ? "active" : ""}`} aria-pressed={filters.favorites} onClick={() => update({ favorites: !filters.favorites })}><Heart />{isRu ? "Избранное" : "Favorites"}</button>
        <button className={`s-filter-button ${filters.selected ? "active" : ""}`} aria-pressed={filters.selected} onClick={() => update({ selected: !filters.selected })}><CheckCheck />{isRu ? "В сборке" : "Selected"}<span>{countInDomain}</span></button>
      </div>
      <div className="s-category-row"><div className="s-category-tabs" role="group" aria-label={isRu ? "Категория" : "Category"}>{groups.map(group => <button key={group.id} aria-pressed={filters.group === group.id} onClick={() => update({ group: group.id, category: "all" })}>{group[language]}</button>)}</div>{filters.domain === "wardrobe" && <label className="s-select"><SlidersHorizontal /><select aria-label={isRu ? "Тип предмета" : "Item type"} value={filters.category} onChange={event => update({ category: event.target.value })}><option value="all">{isRu ? "Все типы" : "All types"}</option>{categories.map(category => <option key={category} value={category}>{categoryLabel(category, language)}</option>)}</select><ChevronDown /></label>}</div>
    </div>
    <div className="s-delivery-note" role="note"><span className={filters.domain === "game" ? "is-pilot" : "is-preview"}>{deliveryLabel(filters.domain === "game" ? "pilot" : "preview", language)}</span><p>{filters.domain === "game" ? (isRu ? "Три закреплённых мода — Tree Mod, Networth и Unit Query HUD — доступны в этом статусе. Остальные карточки сохраняются в набор, но пока не записываются в Dota 2." : "Three pinned mods — Tree Mod, Networth and Unit Query HUD — have this status. Other cards can be saved to a build but are not written to Dota 2 yet.") : (isRu ? "Облики и эффекты можно изучать и сохранять в наборы. Их установка в Dota 2 пока не включена." : "Looks and effects can be explored and saved to builds. Installing them into Dota 2 is not enabled yet.")}</p></div>
    <div className="s-results-head"><span>{modCount(results.length, language)}{filters.query && <> · «{filters.query}»</>}</span>{activeFilter && <button className="s-text-button" onClick={() => setFilters({ ...initialFilters, domain: filters.domain })}>{isRu ? "Сбросить фильтры" : "Reset filters"}<X /></button>}<label className="s-sort"><select aria-label={isRu ? "Сортировка" : "Sort order"} value={filters.sort} onChange={event => update({ sort: event.target.value })}><option value="featured">{isRu ? "Сначала с превью" : "Previews first"}</option><option value="new">{isRu ? "Сначала новые" : "Newest first"}</option><option value="name">{isRu ? "По названию" : "Name A–Z"}</option></select><ChevronDown /></label></div>
    {results.length ? <><div className={`s-mod-grid ${filters.domain === "game" ? "is-game" : ""}`}>{results.slice(0, filters.limit).map(mod => <ModCard key={mod.id} mod={mod} selected={selected.includes(mod.id)} onOpen={() => onOpen(mod)} onToggle={() => onToggle(mod)} />)}</div>{results.length > filters.limit && <button className="s-load-more s-btn" onClick={() => update({ limit: filters.limit + 24 })}>{isRu ? "Показать ещё" : "Show more"}<span>{Math.min(24, results.length - filters.limit)}</span><ArrowRight /></button>}</> : <Empty icon={<Search />} title={isRu ? "Здесь пока ничего нет" : "Nothing here yet"} text={filters.favorites ? (isRu ? "Отмечай понравившиеся моды сердечком в карточке." : "Save mods with the heart in their detail panel.") : (isRu ? "Попробуй другое название или убери часть фильтров." : "Try another name or remove some filters.")} action={isRu ? "Сбросить фильтры" : "Reset filters"} onAction={() => setFilters({ ...initialFilters, domain: filters.domain })} />}
  </div>;
}

export function ModDetails({ mod, selected, favorite, variant, onVariant, onClose, onToggle, onFavorite }: { mod: StudioMod; selected: boolean; favorite: boolean; variant: number; onVariant: (variant: number) => void; onClose: () => void; onToggle: () => void; onFavorite: () => void }) {
  const { language, isRu } = useLocale();
  const [infoOpen, setInfoOpen] = useState(false);
  const preview = mod.variants[variant]?.image ?? mod.image;
  const pilot = isPilotMod(mod.id);
  return <Modal className="s-detail-dialog" title={isRu ? "Подробнее о моде" : "Mod details"} onClose={onClose}>
    <div className="s-detail-media"><Media src={preview ?? undefined} showPending pendingLabel={mod.categoryName[language]} /><span className="s-detail-category">{mod.categoryName[language]}</span></div>
    <div className="s-detail-content"><div className="s-detail-title"><div><span className="s-eyebrow">{mod.author}</span><h2>{mod.name[language]}</h2></div><button className={`s-icon s-favorite ${favorite ? "active" : ""}`} aria-label={isRu ? "В избранное" : "Save to favorites"} aria-pressed={favorite} onClick={onFavorite}><Heart /></button></div>
      <p className="s-detail-description">{mod.description[language].split("\n\n")[0]}</p>
      {mod.variants.length > 1 && <fieldset className="s-variants"><legend>{isRu ? "Вариант" : "Style"}</legend>{mod.variants.map((style, index) => <button key={`${style.name}-${index}`} aria-pressed={variant === index} onClick={() => onVariant(index)}><i style={{ background: style.color }} />{style.name}{variant === index && <Check />}</button>)}</fieldset>}
      {mod.domain === "wardrobe" && mod.group === "audio" && <div className="s-inline-note"><Volume2 /><p>{isRu ? "Послушай оригинал перед выбором. Изображение не передаёт звучание." : "Listen to the original before choosing. The image cannot preview audio."}</p><a href={mod.source} target="_blank" rel="noreferrer"><ExternalLink /></a></div>}
      <div className="s-detail-facts"><div><span>{isRu ? "Раздел" : "Section"}</span><strong>{mod.domain === "game" ? (isRu ? "Настройка игры" : "Game tuning") : (isRu ? "Облики и эффекты" : "Looks & effects")}</strong></div><div><span>{isRu ? "Статус" : "Status"}</span><strong className={pilot ? "s-status-pilot" : "s-status-preview"}>{deliveryLabel(pilot ? "pilot" : "preview", language)}</strong></div><div><span>{isRu ? "Автор / источник" : "Creator / source"}</span><strong>{mod.author}</strong></div></div>
      <button className="s-disclosure" aria-expanded={infoOpen} onClick={() => setInfoOpen(!infoOpen)}>{isRu ? "Источник и совместимость" : "Source & compatibility"}<ChevronDown className={infoOpen ? "rotated" : ""} /></button>{infoOpen && <div className="s-disclosed"><p>{isRu ? "Совместимость с текущей версией Dota ещё не проверена. Превью и сведения взяты из исходного каталога." : "Compatibility with the current Dota version has not been verified. Preview and metadata come from the source catalog."}</p><a href={mod.source} target="_blank" rel="noreferrer">{isRu ? "Открыть источник" : "Open source"}<ExternalLink /></a></div>}
    </div>
    <footer className="s-detail-footer"><button className={`s-btn ${selected ? "s-btn-selected" : "s-btn-primary"}`} onClick={onToggle}>{selected ? <Check /> : <Plus />}{selected ? (isRu ? "Убрать из сборки" : "Remove from build") : (isRu ? "Добавить в сборку" : "Add to build")}</button><small><Layers3 />{pilot ? (isRu ? "Установка доступна в «Моей сборке» на Windows" : "Installation is available in My build on Windows") : (isRu ? "Сохранится в наборе; установка пока недоступна" : "Saved to your build; installation is not available yet")}</small></footer>
  </Modal>;
}
