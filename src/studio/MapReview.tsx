import { TriangleAlert } from "lucide-react";
import type { TreePilotPlan } from "../engine";
import { useLocale } from "../i18n";
import { formatBytes } from "./WardrobeReview";

/**
 * What a map terrain does to the game, before it is installed, in plain
 * words: it is a whole map, where it shows, the one game setting it needs,
 * and how to undo it.
 */
export function MapReview({
  plan,
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
  return (
    <section
      className="b-wardrobe b-map"
      aria-label={isRu ? "Что делает карта" : "What the map does"}
    >
      <h3>{isRu ? "Карта в сборке" : "Map in the build"}</h3>
      <article className="b-wardrobe-item">
        <header>
          <strong>{packageLabel(map.packageId)}</strong>
        </header>
        <dl className="b-facts">
          <div>
            <dt>{isRu ? "Размер" : "Size"}</dt>
            <dd>{formatBytes(map.bytes, isRu)}</dd>
          </div>
        </dl>
        <p className="b-pilot-hint">
          {isRu
            ? "Это целая карта Dota, а не облик. Одновременно стоит только одна карта."
            : "This is a whole Dota map, not a skin. Only one map is installed at a time."}
        </p>
        <div className="b-pilot-blocker is-warning" role="note">
          <TriangleAlert />
          <div>
            <strong>{isRu ? "Что нужно знать" : "Good to know"}</strong>
            <ul className="b-wardrobe-paths">
              <li>
                {isRu
                  ? "Карта видна только в матчах, в режиме «Демо» её нет."
                  : "The map shows only in matches, not in Demo mode."}
              </li>
              <li>
                {isRu
                  ? "Отключи террейн Dota+ в настройках игры, иначе он перекроет карту."
                  : "Turn off the Dota+ terrain in the game settings, or it covers the map."}
              </li>
              <li>
                {isRu
                  ? "После крупного обновления Dota карту может понадобиться переустановить. Если игра ведёт себя странно, нажми «Откатить сборку»."
                  : "After a major Dota update the map may need to be installed again. If the game acts strangely, use Restore the game."}
              </li>
            </ul>
          </div>
        </div>
      </article>
    </section>
  );
}
