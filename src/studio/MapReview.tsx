import { TriangleAlert } from "lucide-react";
import type { TreePilotPlan } from "../engine";
import { useLocale } from "../i18n";
import { formatBytes } from "./WardrobeReview";

function permissionNote(permission: string, isRu: boolean) {
  if (permission === "valve_content_mirrored_by_catalog")
    return isRu
      ? "Это карта из самой Dota 2, которую зеркалит каталог проекта-источника. BetterFy её не хранит и права на неё не проверял."
      : "This is a map from Dota 2 itself, mirrored by the source project's catalog. BetterFy does not host it and has not checked the rights to it.";
  return permission;
}

/** The file types of a map package, most frequent first, as `type count`. */
function typeSummary(extensions: Record<string, number>) {
  return Object.entries(extensions)
    .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
    .map(([type, count]) => `${type} ${count}`)
    .join(" · ");
}

/**
 * What a map terrain does to the game, before it is installed: where the one
 * file goes, what the audit found in it, and what the source project's own
 * guide says about it. A map replaces the game's map package as a whole, so
 * the review says that plainly and keeps the unverified risk apart from the
 * facts.
 */
export function MapReview({
  plan,
  language,
  packageLabel,
}: {
  plan: TreePilotPlan;
  /** The Dota language folder the build is going to, once chosen. */
  language: string;
  packageLabel: (id: string) => string;
}) {
  const { isRu } = useLocale();
  const map = plan.map;
  if (!map) return null;
  const folder = language ? `game/dota_${language}` : "game/dota_<language>";
  return (
    <section className="b-wardrobe b-map" aria-label={isRu ? "Что делает карта" : "What the map does"}>
      <h3>{isRu ? "Карта в сборке" : "Map in the build"}</h3>
      <article className="b-wardrobe-item">
        <header>
          <strong>{packageLabel(map.packageId)}</strong>
          <code>{map.file}</code>
        </header>
        <dl className="b-facts">
          <div>
            <dt>{isRu ? "Куда запишется" : "Where it is written"}</dt>
            <dd>
              <code>
                {folder}/{map.file}
              </code>
            </dd>
          </div>
          <div>
            <dt>{isRu ? "Размер файла" : "File size"}</dt>
            <dd>{formatBytes(map.bytes, isRu)}</dd>
          </div>
          <div>
            <dt>{isRu ? "Файлов внутри пакета карты" : "Files inside the map package"}</dt>
            <dd>{map.report.entries}</dd>
          </div>
          <div>
            <dt>{isRu ? "Файлы Valve в списке pak01" : "Files also in Valve's pak01 list"}</dt>
            <dd>
              {map.valveOverlap === 0
                ? isRu
                  ? `нет (проверено ${map.report.entries} путей)`
                  : `none (${map.report.entries} paths checked)`
                : String(map.valveOverlap)}
            </dd>
          </div>
          <div>
            <dt>{isRu ? "Карта SHA-256" : "Map SHA-256"}</dt>
            <dd>
              <code>{map.sha256.slice(0, 16)}…</code>
            </dd>
          </div>
          <div>
            <dt>{isRu ? "Архив SHA-256" : "Archive SHA-256"}</dt>
            <dd>
              <code>{map.archiveSha256.slice(0, 16)}…</code>
            </dd>
          </div>
        </dl>
        <p className="b-pilot-hint">
          {isRu
            ? "Это не облик, а целая карта Dota. BetterFy кладёт один файл, maps/dota.vpk, в папку языка, как велит инструкция проекта-источника. Он не объединяется с остальной сборкой: облики героев по-прежнему лежат в своём pak-файле рядом. Одновременно ставится только одна карта."
            : "This is not a skin but a whole Dota map. BetterFy writes one file, maps/dota.vpk, into the language folder, as the source project's guide says. It is not merged into the rest of the build: hero skins still sit in their own pak file beside it. Only one map is installed at a time."}
        </p>
        <div className="b-pilot-blocker is-warning" role="note">
          <TriangleAlert />
          <div>
            <strong>
              {isRu ? "Что нужно знать про карты" : "What to know about maps"}
            </strong>
            <ul className="b-wardrobe-paths">
              <li>
                {isRu
                  ? "Карта видна только в матчах, в режиме «Демо» её нет. Так сказано в инструкции проекта-источника."
                  : "The map shows only in matches, not in Demo mode. That is what the source project's guide says."}
              </li>
              <li>
                {isRu
                  ? "Отключи террейн Dota+ в настройках игры, иначе он конфликтует с картой (по той же инструкции)."
                  : "Turn off the Dota+ terrain in the game settings, or it conflicts with the map (same guide)."}
              </li>
              <li>
                {isRu
                  ? "Карта заменяет файл карты целиком. После патча Dota такой файл может перестать подходить игре. Это непроверенный риск, а не установленный факт: если игра ведёт себя странно, нажми «Откатить сборку»."
                  : "The map replaces the game's map file as a whole. After a Dota patch such a file may stop fitting the game. That is an unverified risk, not an established fact: if the game acts strangely, use Restore the game."}
              </li>
              <li>
                {isRu
                  ? "Установку карты через BetterFy в Dota ещё не запускали. Основатель сообщает, что VPK этого каталога, положенные в папку языка вручную, работают; отдельного отчёта именно про карты нет."
                  : "BetterFy's install of a map has not been run in Dota yet. The founder reports that this catalog's VPKs, copied into the language folder by hand, work; there is no separate report about maps."}
              </li>
            </ul>
          </div>
        </div>
        <p className="b-pilot-hint">
          {isRu
            ? "Если в папке уже лежит чужой maps/dota.vpk, BetterFy его не тронет: установка остановится, пока ты сам не уберёшь файл."
            : "If a maps/dota.vpk that BetterFy did not write is already there, BetterFy leaves it alone: the install stops until you move that file yourself."}
        </p>
        <p className="b-pilot-hint">{permissionNote(map.permission, isRu)}</p>
        <details className="b-wardrobe-list">
          <summary>
            {isRu ? "Что внутри пакета карты" : "What is inside the map package"}
          </summary>
          <p className="b-pilot-hint">
            {isRu
              ? `Формат VPK v${map.report.vpkVersion}, все файлы внутри одного файла. Типы файлов: ${typeSummary(map.report.extensions)}. Скриптов и программ в пакете нет: это проверка по типам файлов, она не говорит, как карта поведёт себя в игре.`
              : `VPK v${map.report.vpkVersion}, every file inside the one file. File types: ${typeSummary(map.report.extensions)}. The package holds no scripts or programs: that is a check on file types and does not say how the map behaves in the game.`}
          </p>
        </details>
      </article>
    </section>
  );
}
