import { TriangleAlert } from "lucide-react";
import type { TreePilotPlan, WardrobePlanItem } from "../engine";
import { useLocale } from "../i18n";
import { categoryHelp } from "./categoryHelp";
import { categoryLabel } from "./model";

/** KB under a megabyte, MB above: builds with a skin are tens of megabytes. */
export function formatBytes(bytes: number, isRu: boolean) {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} ${isRu ? "КБ" : "KB"}`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} ${isRu ? "МБ" : "MB"}`;
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
  // `strip` is the opt-in: the shared game files stay out of the build.
  const keepShared = isHero && item.sharedPolicy === "strip";
  const sharedCount = report.shared.files;
  return (
    <article className="b-wardrobe-item">
      <header>
        <strong>{label}</strong>
        <span>{isHero ? item.heroes.join(" · ") : categoryLabel(item.category, language)}</span>
      </header>
      <dl className="b-facts">
        <div>
          <dt>{isRu ? "Размер в сборке" : "Size in the build"}</dt>
          <dd>{formatBytes(report.install.bytes, isRu)}</dd>
        </div>
      </dl>
      <p className="b-pilot-hint">{help.installs[language]}</p>
      {help.tip && <p className="b-pilot-hint">{help.tip[language]}</p>}
      {isHero && onToggleKeepShared && (sharedCount > 0 || keepShared) && (
        <label className="b-check">
          <input
            type="checkbox"
            checked={keepShared}
            disabled={disabled}
            onChange={(event) => onToggleKeepShared(item.packageId, event.target.checked)}
          />
          <span>
            {isRu ? "Менять только этого героя" : "Change only this hero"}
            <small>
              {isRu
                ? "Облик не тронет эффекты и текстуры других героев и интерфейса. Он может выглядеть чуть иначе, чем у автора."
                : "The skin leaves other heroes' and the interface's effects and textures alone. It may look slightly different from the author's version."}
            </small>
          </span>
        </label>
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
  // Who wins over whom, once per pair of items rather than once per file.
  const pairs = [
    ...conflicts
      .reduce((winners, entry) => {
        const others = winners.get(entry.winnerPackageId) ?? new Set<string>();
        entry.shadowedPackageIds.forEach((id) => others.add(id));
        return winners.set(entry.winnerPackageId, others);
      }, new Map<string, Set<string>>())
      .entries(),
  ].map(([winner, others]) => [winner, [...others]] as const);
  return (
    <section
      className="b-wardrobe"
      aria-label={isRu ? "Что добавляют облики и пакеты" : "What the skins and packs add"}
    >
      <h3>{isRu ? "Облики и пакеты в сборке" : "Skins and packs in the build"}</h3>
      <p className="b-pilot-hint">
        {isRu
          ? "Откатить можно в любой момент одной кнопкой: BetterFy вернёт игру как было."
          : "You can restore at any time with one button: BetterFy puts the game back as it was."}
      </p>
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
                ? "Несколько обликов меняют одни и те же файлы"
                : "Several looks change the same files"}
            </strong>
            <p>
              {isRu
                ? "В игре останется вариант того, кто выше в списке сборки."
                : "The game keeps the version of the one higher in the build list."}
            </p>
            <ul className="b-wardrobe-paths">
              {pairs.map(([winner, others]) => (
                <li key={winner}>
                  <span>
                    {packageLabel(winner)} → {others.map(packageLabel).join(", ")}
                  </span>
                </li>
              ))}
            </ul>
            <label className="b-check">
              <input
                type="checkbox"
                checked={acknowledged}
                disabled={disabled}
                onChange={(event) => onAcknowledge(event.target.checked)}
              />
              <span>{isRu ? "Понятно, оставляем так" : "Got it, keep it this way"}</span>
            </label>
          </div>
        </div>
      )}
    </section>
  );
}

/**
 * Cursor and font packs of the build: they replace the game's own files in
 * its folders, so the review says that and how it is undone.
 */
export function PackReview({
  plan,
  packageLabel,
}: {
  plan: TreePilotPlan;
  packageLabel: (id: string) => string;
}) {
  const { isRu } = useLocale();
  const packs = plan.packs ?? [];
  if (packs.length === 0) return null;
  return (
    <section className="b-wardrobe" aria-label={isRu ? "Курсоры и шрифты" : "Cursors and fonts"}>
      <h3>{isRu ? "Курсоры и шрифты" : "Cursors and fonts"}</h3>
      {packs.map((pack) => (
        <article className="b-wardrobe-item" key={pack.packageId}>
          <header>
            <strong>{packageLabel(pack.packageId)}</strong>
            <span>
              {pack.category === "fonts"
                ? isRu
                  ? "Шрифты"
                  : "Fonts"
                : isRu
                  ? "Курсор"
                  : "Cursor"}
            </span>
          </header>
          <dl className="b-facts">
            <div>
              <dt>{isRu ? "Размер" : "Size"}</dt>
              <dd>{formatBytes(pack.bytes, isRu)}</dd>
            </div>
          </dl>
          <p className="b-pilot-hint">
            {pack.category === "fonts"
              ? isRu
                ? "Шрифты игры уходят в резервную копию, шрифты пака встают на их место."
                : "The game's fonts go into a backup and the pack's fonts take their place."
              : isRu
                ? "Файлы курсора игры уходят в резервную копию, файлы пака встают на их место."
                : "The game's cursor files go into a backup and the pack's take their place."}
          </p>
        </article>
      ))}
      <p className="b-pilot-hint">
        {isRu
          ? "Откат вернёт файлы игры. Если Dota после обновления вернёт их сама, просто поставь сборку снова."
          : "Restore puts the game's files back. If a Dota update puts them back first, install the build again."}
      </p>
    </section>
  );
}
