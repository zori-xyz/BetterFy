import { TriangleAlert } from "lucide-react";
import type { SkinRejectReason, TreePilotPlan, WardrobePlanItem } from "../engine";
import { useLocale } from "../i18n";

/** KB under a megabyte, MB above: builds with a skin are tens of megabytes. */
export function formatBytes(bytes: number, isRu: boolean) {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} ${isRu ? "КБ" : "KB"}`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} ${isRu ? "МБ" : "MB"}`;
}

const reasons: Record<SkinRejectReason, [string, string]> = {
  path_unsafe: ["небезопасный путь", "unsafe path"],
  duplicate_path: ["путь повторяется", "duplicate path"],
  oversized: ["слишком большой файл", "file too large"],
  empty_resource: ["пустой файл", "empty file"],
  script: ["скрипт", "script"],
  uncompiled_source: ["исходник, не скомпилирован", "uncompiled source"],
  executable: ["программа", "executable"],
  type_not_allowed: ["тип файла не разрешён", "file type not allowed"],
  panorama_unproven: ["файл интерфейса не подтверждён", "interface file not proven"],
};

const MAX_LISTED_CONFLICTS = 20;

function permissionNote(permission: string, isRu: boolean) {
  if (permission === "founder_reported_author_consent")
    return isRu
      ? "Авторы, по словам основателя проекта, разрешили использование сообществом. Файла лицензии в архиве нет."
      : "The authors agreed to community use, as reported by the project's founder. The archive carries no licence file.";
  return permission;
}

function Item({ item, label }: { item: WardrobePlanItem; label: string }) {
  const { isRu } = useLocale();
  const { report } = item;
  const strip = report.sharedPolicy === "strip";
  return (
    <article className="b-wardrobe-item">
      <header>
        <strong>{label}</strong>
        <code>{item.hero}</code>
        {strip && report.shared.files > 0 && (
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
        <div>
          <dt>{isRu ? "Файлы героя" : "Hero files"}</dt>
          <dd>{report.heroScoped.files}</dd>
        </div>
        <div>
          <dt>{isRu ? "Файлы автора" : "Author's files"}</dt>
          <dd>{report.authorOwned.files}</dd>
        </div>
        <div>
          <dt>{isRu ? "Общие файлы игры" : "Shared game files"}</dt>
          <dd>
            {report.shared.files === 0
              ? isRu
                ? "нет"
                : "none"
              : strip
                ? isRu
                  ? `${report.shared.files} убрано`
                  : `${report.shared.files} left out`
                : isRu
                  ? `${report.shared.files}, установка отклонена`
                  : `${report.shared.files}, install refused`}
          </dd>
        </div>
        <div>
          <dt>{isRu ? "Отброшено" : "Dropped"}</dt>
          <dd>{report.rejected.files}</dd>
        </div>
        <div>
          <dt>{isRu ? "Архив SHA-256" : "Archive SHA-256"}</dt>
          <dd>
            <code>{item.archiveSha256.slice(0, 16)}…</code>
          </dd>
        </div>
      </dl>
      <p className="b-pilot-hint">
        {isRu
          ? "Общие файлы игры (эффекты и текстуры других героев и интерфейса) BetterFy не трогает: они убраны из сборки и перечислены ниже. Выглядит ли облик правильно без них, и работает ли он в игре, ещё не проверено."
          : "BetterFy leaves shared game files (other heroes' and interface effects and textures) alone: they are removed from the build and listed below. Whether the skin looks right without them, or works in the game at all, has not been checked."}
      </p>
      <p className="b-pilot-hint">{permissionNote(item.permission, isRu)}</p>
      {report.sharedPaths.length > 0 && (
        <details className="b-wardrobe-list">
          <summary>
            {isRu
              ? `Убранные общие файлы (${report.sharedPaths.length})`
              : `Shared files left out (${report.sharedPaths.length})`}
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
      {report.rejectedPaths.length > 0 && (
        <details className="b-wardrobe-list">
          <summary>
            {isRu
              ? `Отброшенные файлы (${report.rejectedPaths.length})`
              : `Dropped files (${report.rejectedPaths.length})`}
          </summary>
          <ul>
            {report.rejectedPaths.map((entry) => (
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
 * and the paths a skin shares with another item (those need a confirmation
 * before the install button works).
 */
export function WardrobeReview({
  plan,
  packageLabel,
  acknowledged,
  disabled,
  onAcknowledge,
}: {
  plan: TreePilotPlan;
  packageLabel: (id: string) => string;
  acknowledged: boolean;
  disabled: boolean;
  onAcknowledge: (value: boolean) => void;
}) {
  const { isRu } = useLocale();
  if (plan.wardrobe.length === 0) return null;
  const conflicts = plan.wardrobeConflicts;
  return (
    <section
      className="b-wardrobe"
      aria-label={isRu ? "Что добавляет облик" : "What the skin adds"}
    >
      <h3>{isRu ? "Облик в сборке" : "Skin in the build"}</h3>
      {plan.wardrobe.map((item) => (
        <Item item={item} key={item.packageId} label={packageLabel(item.packageId)} />
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
