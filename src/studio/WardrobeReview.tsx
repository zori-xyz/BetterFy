import { TriangleAlert } from "lucide-react";
import type { SkinRejectReason, TreePilotPlan, WardrobePlanItem } from "../engine";
import { useLocale } from "../i18n";
import { categoryHelp, panoramaReview } from "./categoryHelp";
import { categoryLabel } from "./model";

/** KB under a megabyte, MB above: builds with a skin are tens of megabytes. */
export function formatBytes(bytes: number, isRu: boolean) {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} ${isRu ? "КБ" : "KB"}`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} ${isRu ? "МБ" : "MB"}`;
}

const reasons: Record<SkinRejectReason, [string, string]> = {
  path_unsafe: ["небезопасный путь", "unsafe path"],
  duplicate_path: ["путь повторяется", "duplicate path"],
  oversized: ["слишком большой файл", "file too large"],
  script: ["скрипт", "script"],
  uncompiled_source: ["исходник, не скомпилирован", "uncompiled source"],
  not_game_file: ["служебный файл, не игровой", "tool or reading file, not a game file"],
  executable: ["программа", "executable"],
  type_not_allowed: ["тип файла не устанавливается", "file type not installed"],
  panorama_scripted: [
    "в файле интерфейса есть скрипт или обработчик событий",
    "interface file carries a script, event handler or include",
  ],
  panorama_unproven: ["файл интерфейса не подтверждён", "interface file not proven"],
  outside_category: [
    "вне папок, из которых ставится эта категория",
    "outside the folders this category installs from",
  ],
  decor_refused: [
    "проверка макета, стиля или видео не подтвердила файл",
    "the layout, style or video check did not prove this file",
  ],
};

/** Reasons for files that are not game files: leaving them out changes nothing in game. */
const IGNORED_REASONS: ReadonlySet<SkinRejectReason> = new Set([
  "uncompiled_source",
  "not_game_file",
]);

const MAX_LISTED_CONFLICTS = 20;

function permissionNote(permission: string, isRu: boolean) {
  if (permission === "founder_reported_author_consent")
    return isRu
      ? "Авторы, по словам основателя проекта, разрешили использование сообществом. Файла лицензии в архиве нет."
      : "The authors agreed to community use, as reported by the project's founder. The archive carries no licence file.";
  return permission;
}

function Item({
  item,
  label,
  disabled,
  onToggleKeepShared,
}: {
  item: WardrobePlanItem;
  label: string;
  disabled: boolean;
  onToggleKeepShared?: (packageId: string, keep: boolean) => void;
}) {
  const { isRu, language } = useLocale();
  const { report } = item;
  // Only hero skins have the opt-in mode; every other category installs whole.
  const isHero = item.scope === "hero";
  const help = categoryHelp(item.category);
  const panorama = panoramaReview(item.category);
  // `strip` is the opt-in: the shared game files stay out of the build.
  const keepShared = isHero && item.sharedPolicy === "strip";
  const sharedCount = report.shared.files;
  const dropped = report.rejectedPaths.filter((entry) => !IGNORED_REASONS.has(entry.reason));
  const ignored = report.rejectedPaths.filter((entry) => IGNORED_REASONS.has(entry.reason));
  return (
    <article className="b-wardrobe-item">
      <header>
        <strong>{label}</strong>
        <code>{isHero ? item.heroes.join(" · ") : categoryLabel(item.category, language)}</code>
        {keepShared && (
          <small>
            {isRu
              ? "Чужой лут не трогаем: общие файлы остаются как в игре."
              : "Hands off everyone else's loot: shared files stay as the game has them."}
          </small>
        )}
      </header>
      <dl className="b-facts">
        <div>
          <dt>{isRu ? "В сборку" : "Into the build"}</dt>
          <dd>
            {report.install.files} {isRu ? "файлов" : "files"} ·{" "}
            {formatBytes(report.install.bytes, isRu)}
          </dd>
        </div>
        {isHero && (
          <div>
            <dt>{isRu ? "Файлы героя" : "Hero files"}</dt>
            <dd>{report.heroScoped.files}</dd>
          </div>
        )}
        <div>
          <dt>{isRu ? "Новые файлы (в игре таких нет)" : "New files (not in the game)"}</dt>
          <dd>{report.authorOwned.files}</dd>
        </div>
        <div>
          <dt>
            {keepShared
              ? isRu
                ? "Общие файлы игры"
                : "Shared game files"
              : !isHero
                ? isRu
                  ? "Заменяет файлы игры"
                  : "Replaces game files"
                : isRu
                  ? "Заодно заменяет общие файлы игры"
                  : "Also replaces shared game files"}
          </dt>
          <dd>
            {sharedCount === 0
              ? isRu
                ? "нет"
                : "none"
              : keepShared
                ? isRu
                  ? `${sharedCount} не трогаем`
                  : `${sharedCount} left untouched`
                : String(sharedCount)}
          </dd>
        </div>
        <div>
          <dt>{isRu ? "Отброшено" : "Dropped"}</dt>
          <dd>{report.rejected.files}</dd>
        </div>
        {report.ignored.files > 0 && (
          <div>
            <dt>{isRu ? "Не игровые файлы, пропущены" : "Not game files, ignored"}</dt>
            <dd>{report.ignored.files}</dd>
          </div>
        )}
        {report.renamed > 0 && (
          <div>
            <dt>{isRu ? "Имена в нижний регистр" : "Names lower-cased"}</dt>
            <dd>{report.renamed}</dd>
          </div>
        )}
        <div>
          <dt>{isRu ? "Архив SHA-256" : "Archive SHA-256"}</dt>
          <dd>
            <code>{item.archiveSha256.slice(0, 16)}…</code>
          </dd>
        </div>
      </dl>
      <p className="b-pilot-hint">{help.installs[language]}</p>
      {help.tip && <p className="b-pilot-hint">{help.tip[language]}</p>}
      <p className="b-pilot-hint">
        {keepShared
          ? isRu
            ? "Общие файлы игры (эффекты и текстуры других героев и интерфейса) не попадают в сборку и перечислены ниже. Это может изменить внешний вид облика: часть текстур и материалов не загрузится или будет игровой. Как он выглядит так, и работает ли в игре, не проверено."
            : "Shared game files (other heroes' and interface effects and textures) are kept out of the build and listed below. This can change how the skin looks: some textures and materials may be missing or fall back to the game's own. How it looks this way, and whether it works in the game at all, has not been checked."
          : !isHero
            ? isRu
              ? "В сборку идёт всё, что есть в архиве, как при ручном копировании VPK в папку игры; файлы, которые заменяют файлы игры, перечислены ниже. Основатель сообщает, что оригинальные VPK, скопированные так вручную, работали. В Dota этот пакет через BetterFy ещё не запускали: имя файла BetterFy выбирает сам."
              : "Everything the archive carries goes into the build, as when its VPK is copied into the game folder by hand; the files that replace game files are listed below. The founder reports that original VPKs copied that way by hand worked. This pack has not been run in Dota through BetterFy yet: BetterFy picks the file name itself."
            : isRu
              ? "В сборку идёт всё, что есть в архиве, как при ручном копировании VPK в папку игры: свои файлы облика и общие файлы игры, которые он заменяет (они перечислены ниже). Основатель сообщает, что скины, скопированные так на Windows, выглядели правильно; установку этого облика через BetterFy в Dota ещё не запускали."
              : "Everything the archive carries goes into the build, as when its VPK is copied into the game folder by hand: the skin's own files and the shared game files it replaces (listed below). The founder reports that skins copied this way looked right on Windows; BetterFy's install of this skin has not been run in Dota yet."}
      </p>
      {panorama && (
        <>
          <p className="b-pilot-hint">{panorama.proved[language]}</p>
          <p className="b-pilot-hint">{panorama.notProved[language]}</p>
          <p className="b-pilot-hint">{panorama.undo[language]}</p>
        </>
      )}
      <p className="b-pilot-hint">{permissionNote(item.permission, isRu)}</p>
      {isHero && onToggleKeepShared && (sharedCount > 0 || keepShared) && (
        <label className="b-check">
          <input
            type="checkbox"
            checked={keepShared}
            disabled={disabled}
            onChange={(event) => onToggleKeepShared(item.packageId, event.target.checked)}
          />
          <span>
            {isRu
              ? "Не трогать общие файлы игры"
              : "Keep the game's shared files untouched"}
            <small>
              {isRu
                ? `Уберёт из сборки ${sharedCount} общих файлов. Облик может выглядеть иначе или неполно. План будет подготовлен заново.`
                : `Leaves ${sharedCount} shared files out of the build. The skin may look different or incomplete. The plan is prepared again.`}
            </small>
          </span>
        </label>
      )}
      {report.sharedPaths.length > 0 && (
        <details className="b-wardrobe-list">
          <summary>
            {keepShared
              ? isRu
                ? `Общие файлы игры, не вошедшие в сборку (${report.sharedPaths.length})`
                : `Shared game files left out (${report.sharedPaths.length})`
              : !isHero
                ? isRu
                  ? `Файлы игры, которые он заменяет (${report.sharedPaths.length})`
                  : `Game files it replaces (${report.sharedPaths.length})`
                : isRu
                  ? `Общие файлы игры, которые он заменяет (${report.sharedPaths.length})`
                  : `Shared game files it replaces (${report.sharedPaths.length})`}
          </summary>
          <ul>
            {report.sharedPaths.map((entry) => (
              <li key={entry.path}>
                <code>{entry.path}</code>
              </li>
            ))}
          </ul>
        </details>
      )}
      {item.gameChecked && report.newInGame.length > 0 && (
        <details className="b-wardrobe-list">
          <summary>
            {isRu
              ? `Файлы облика, которые теперь есть и в вашей игре (${report.newInGame.length})`
              : `Skin files your game ships now (${report.newInGame.length})`}
          </summary>
          <p className="b-pilot-hint">
            {isRu
              ? "При аудите их в списке Valve не было, а в вашей Dota они уже есть: облик заменит их. Это справочная информация."
              : "They were not in Valve's list at audit time, but your Dota has them now, so the skin replaces them. Information only."}
          </p>
          <ul>
            {report.newInGame.map((path) => (
              <li key={path}>
                <code>{path}</code>
              </li>
            ))}
          </ul>
        </details>
      )}
      {!item.gameChecked && (
        <p className="b-pilot-hint">
          {isRu
            ? "Файлы облика не сверялись со списком файлов вашей Dota (папка игры не подключена или не читается)."
            : "The skin's files were not compared with your own Dota file list (no game folder connected, or it could not be read)."}
        </p>
      )}
      {dropped.length > 0 && (
        <details className="b-wardrobe-list">
          <summary>
            {isRu ? `Отброшенные файлы (${dropped.length})` : `Dropped files (${dropped.length})`}
          </summary>
          <ul>
            {dropped.map((entry) => (
              <li key={entry.path}>
                <code>{entry.path}</code>
                <span>{reasons[entry.reason]?.[isRu ? 0 : 1] ?? entry.reason}</span>
                {entry.detail && <code>{entry.detail}</code>}
              </li>
            ))}
          </ul>
        </details>
      )}
      {ignored.length > 0 && (
        <details className="b-wardrobe-list">
          <summary>
            {isRu
              ? `Не игровые файлы, пропущены (${ignored.length})`
              : `Not game files, ignored (${ignored.length})`}
          </summary>
          <p className="b-pilot-hint">
            {isRu
              ? "Сор из мастерской автора: исходники, readme и служебные файлы. Игра грузит только скомпилированные файлы, так что без них облик выглядит так же."
              : "Leftovers from the author's workshop: sources, readmes and tool files. The game loads only compiled files, so leaving these out changes nothing in the game."}
          </p>
          <ul>
            {ignored.map((entry) => (
              <li key={entry.path}>
                <code>{entry.path}</code>
                <span>{reasons[entry.reason]?.[isRu ? 0 : 1] ?? entry.reason}</span>
              </li>
            ))}
          </ul>
        </details>
      )}
    </article>
  );
}

/**
 * What the skin-archive analyzer found for each wardrobe item of the build,
 * the per-item switch that keeps the game's shared files untouched, and the
 * paths a skin shares with another item (those need a confirmation before the
 * install button works).
 */
export function WardrobeReview({
  plan,
  packageLabel,
  acknowledged,
  disabled,
  onAcknowledge,
  onToggleKeepShared,
}: {
  plan: TreePilotPlan;
  packageLabel: (id: string) => string;
  acknowledged: boolean;
  disabled: boolean;
  onAcknowledge: (value: boolean) => void;
  /** Re-prepares the plan with the item in (`true`) or out of the opt-in mode. */
  onToggleKeepShared?: (packageId: string, keep: boolean) => void;
}) {
  const { isRu } = useLocale();
  if (plan.wardrobe.length === 0) return null;
  const conflicts = plan.wardrobeConflicts;
  return (
    <section
      className="b-wardrobe"
      aria-label={isRu ? "Что добавляют облики и пакеты" : "What the skins and packs add"}
    >
      <h3>{isRu ? "Облики и пакеты в сборке" : "Skins and packs in the build"}</h3>
      {plan.wardrobe.map((item) => (
        <Item
          item={item}
          key={item.packageId}
          label={packageLabel(item.packageId)}
          disabled={disabled}
          onToggleKeepShared={onToggleKeepShared}
        />
      ))}
      {conflicts.length > 0 && (
        <div className="b-pilot-blocker is-warning" role="alert">
          <TriangleAlert />
          <div>
            <strong>
              {isRu
                ? `Общие пути с другими элементами сборки: ${conflicts.length}`
                : `Paths shared with other items in the build: ${conflicts.length}`}
            </strong>
            <p>
              {isRu
                ? "Эти файлы записывают сразу несколько элементов, и содержимое у них разное. В сборке остаётся файл того, кто выше в списке."
                : "More than one item writes each of these files, with different contents. The item higher in the list keeps the file."}
            </p>
            <ul className="b-wardrobe-paths">
              {conflicts.slice(0, MAX_LISTED_CONFLICTS).map((entry) => (
                <li key={entry.path}>
                  <code>{entry.path}</code>
                  <span>
                    {packageLabel(entry.winnerPackageId)} →{" "}
                    {entry.shadowedPackageIds.map(packageLabel).join(", ")}
                  </span>
                </li>
              ))}
              {conflicts.length > MAX_LISTED_CONFLICTS && (
                <li>
                  {isRu
                    ? `…и ещё ${conflicts.length - MAX_LISTED_CONFLICTS}`
                    : `…and ${conflicts.length - MAX_LISTED_CONFLICTS} more`}
                </li>
              )}
            </ul>
            <label className="b-check">
              <input
                type="checkbox"
                checked={acknowledged}
                disabled={disabled}
                onChange={(event) => onAcknowledge(event.target.checked)}
              />
              <span>
                {isRu
                  ? "Я посмотрел список и согласен с тем, какие файлы останутся"
                  : "I reviewed the list and accept which files are kept"}
              </span>
            </label>
          </div>
        </div>
      )}
    </section>
  );
}
