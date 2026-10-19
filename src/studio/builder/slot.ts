import type { GameLanguage, LanguageSlotReport, SlotFile } from "../../engine";

// Plain-language reading of the engine's read-only language-folder summary
// (`inspect_language_slot`). It only ever sees file names, sizes and times;
// the wording says "named like Minify's", never that a file is Minify's.

export type SlotExplanation = {
  title: string;
  body: string;
  /** File facts for the "Details" fold, one short line each. */
  details: string[];
  /** The one recommended next step. */
  action: "release" | "report" | "retry" | null;
};

const pick = (isRu: boolean, ru: string, en: string) => (isRu ? ru : en);

/** The refusals that say a file BetterFy recorded no longer matches. */
const isFileCode = (code: string) =>
  code === "deployment_conflict" ||
  code === "rollback_conflict" ||
  code === "deployment_target_foreign";

export const folderOf = (language: GameLanguage) => `dota_${language}`;

const minifyFiles = (report: LanguageSlotReport) =>
  report.files.filter((file) => file.owner === "minify_like");

/** The Minify-named file to mention first: its directory file, else any chunk. */
function minifyHeadline(report: LanguageSlotReport): string | null {
  const files = minifyFiles(report);
  return (files.find((file) => file.name.toLowerCase().endsWith("_dir.vpk")) ?? files[0])?.name ?? null;
}

const formatSize = (bytes: number, isRu: boolean) => {
  const mb = bytes / (1024 * 1024);
  const text = mb >= 10 ? mb.toFixed(0) : mb >= 0.1 ? mb.toFixed(1) : "<0.1";
  return `${text} ${isRu ? "МБ" : "MB"}`;
};

const formatTime = (ms: number | null, isRu: boolean) =>
  ms === null
    ? null
    : new Date(ms).toLocaleString(isRu ? "ru-RU" : "en-GB", {
        dateStyle: "short",
        timeStyle: "short",
      });

const ownerLabel = (file: SlotFile, isRu: boolean) =>
  file.owner === "betterfy"
    ? "BetterFy"
    : file.owner === "minify_like"
      ? pick(isRu, "имя как у Minify", "named like Minify's")
      : pick(isRu, "неизвестный", "unknown");

/** Facts shown under "Details": the folder, BetterFy's record, and each pak file. */
export function slotDetails(report: LanguageSlotReport, isRu: boolean): string[] {
  const lines = [folderOf(report.language)];
  if (report.owned) {
    const state =
      report.owned.state === "intact"
        ? pick(isRu, "на месте", "intact")
        : report.owned.state === "missing"
          ? pick(isRu, "нет в папке", "not in the folder")
          : pick(isRu, "содержимое изменилось", "contents changed");
    lines.push(
      `${pick(isRu, "Запись BetterFy", "BetterFy record")}: ${report.owned.name}, ${state}`,
    );
  }
  for (const file of report.files) {
    const time = formatTime(file.modifiedMs, isRu);
    lines.push(
      [file.name, formatSize(file.bytes, isRu), time, ownerLabel(file, isRu)]
        .filter(Boolean)
        .join(" · "),
    );
  }
  if (report.filesTruncated) lines.push("…");
  if (report.ownedMap || report.map) {
    const state =
      report.ownedMap === "intact"
        ? pick(isRu, "запись BetterFy, на месте", "BetterFy record, intact")
        : report.ownedMap === "missing"
          ? pick(isRu, "запись BetterFy, файла нет", "BetterFy record, file not there")
          : report.ownedMap === "replaced"
            ? pick(isRu, "запись BetterFy, содержимое изменилось", "BetterFy record, contents changed")
            : pick(isRu, "записи BetterFy нет", "no BetterFy record");
    lines.push(
      [
        "maps/dota.vpk",
        report.map ? formatSize(report.map.bytes, isRu) : null,
        formatTime(report.map?.modifiedMs ?? null, isRu),
        state,
      ]
        .filter(Boolean)
        .join(" · "),
    );
  }
  if (report.nextName)
    lines.push(`${pick(isRu, "Следующий файл BetterFy", "BetterFy's next file")}: ${report.nextName}`);
  return lines;
}

/**
 * Before an install: when the folder already holds files named like Minify's,
 * say where BetterFy will put its own. Null when there is nothing to add.
 */
export function slotNote(report: LanguageSlotReport | null, isRu: boolean): string | null {
  if (!report || report.owned) return null;
  const headline = minifyHeadline(report);
  if (!headline) return null;
  const folder = folderOf(report.language);
  if (report.slotFull || !report.nextName)
    return pick(
      isRu,
      `В ${folder} уже есть ${headline} (имя как у Minify), а свободных номеров для BetterFy нет.`,
      `${folder} already has ${headline} (named like Minify's) and no free number is left for BetterFy.`,
    );
  return pick(
    isRu,
    `В ${folder} уже есть ${headline} (имя как у Minify). BetterFy поставит ${report.nextName} рядом, чужие файлы не тронет.`,
    `${folder} already has ${headline} (named like Minify's). BetterFy will use ${report.nextName} next to it and leave other files alone.`,
  );
}

/** The one journal line after an install names the file and what was left alone. */
export function installedLine(
  vpkName: string,
  language: GameLanguage,
  report: LanguageSlotReport | null,
): [ru: string, en: string] {
  const folder = folderOf(language);
  const headline = report ? minifyHeadline(report) : null;
  return headline
    ? [
        `BetterFy использует ${vpkName} в ${folder}; ${headline} (имя как у Minify) не тронут.`,
        `Using ${vpkName} in ${folder}; ${headline} (named like Minify's) is left alone.`,
      ]
    : [`BetterFy использует ${vpkName} в ${folder}.`, `Using ${vpkName} in ${folder}.`];
}

/** The one journal line after an install that wrote a map: which file, where. */
export function installedMapLine(language: GameLanguage): [ru: string, en: string] {
  const folder = folderOf(language);
  return [
    `BetterFy записал карту maps/dota.vpk в ${folder}.`,
    `Wrote the map maps/dota.vpk into ${folder}.`,
  ];
}

/**
 * The refusal codes that come from the language folder not matching BetterFy's
 * record, explained from the folder's actual state. Without a report the
 * wording stays general and still names the likeliest cause.
 */
export function explainSlot(
  code: string,
  isRu: boolean,
  report: LanguageSlotReport | null,
): SlotExplanation | null {
  const folder = report ? folderOf(report.language) : null;
  const details = report ? slotDetails(report, isRu) : [];
  const owned = report?.owned ?? null;
  const ownedMap = report?.ownedMap ?? null;
  const where = folder ?? pick(isRu, "папке языка", "the language folder");

  if (code === "map_target_foreign") {
    const size = report?.map ? ` (${formatSize(report.map.bytes, isRu)})` : "";
    return {
      title: pick(isRu, "В папке уже лежит чужая карта", "A map from someone else is already there"),
      body: pick(
        isRu,
        `В ${where}/maps уже есть файл dota.vpk${size}, и записал его не BetterFy: это может быть карта из другого мода или файл, который положили вручную. BetterFy его не тронул и ничего не записал, потому что не может доказать, что заменённый чужой файл потом вернётся как был. Что делать: убери карту из сборки и поставь остальное, либо сам перенеси этот файл в другое место (он может тебе понадобиться) и нажми «Повторить».`,
        `${where}/maps already has a dota.vpk${size} that BetterFy did not write: it may be a map from another mod or a file put there by hand. BetterFy left it alone and wrote nothing, because it cannot prove that replacing someone else's file could be undone to the exact original. What to do: remove the map from the build and install the rest, or move that file somewhere else yourself (you may want it) and press Retry.`,
      ),
      details,
      action: "retry",
    };
  }

  if (code === "map_folder_invalid") {
    return {
      title: pick(isRu, "Папка maps занята", "The maps folder is in the way"),
      body: pick(
        isRu,
        `В ${where} есть что-то под именем maps, но это не обычная папка (например, файл). BetterFy ничего не записал и ничего не тронул. Переименуй или убери это сам, затем нажми «Повторить».`,
        `${where} has something named maps that is not a plain folder (a file, for example). BetterFy wrote nothing and touched nothing. Rename or remove it yourself, then press Retry.`,
      ),
      details,
      action: "retry",
    };
  }

  if (code === "deployment_path_unsafe") {
    return {
      title: pick(isRu, "Путь в папке игры — это ссылка", "A path in the game folder is a link"),
      body: pick(
        isRu,
        "Одна из папок или один из файлов на пути записи оказались ссылкой (symlink или junction). BetterFy не пишет через ссылки, потому что запись ушла бы в другое место. Ничего не записано и не тронуто. Замени ссылку обычной папкой или файлом и повтори.",
        "A folder or a file on the write path turned out to be a link (a symlink or a junction). BetterFy does not write through links, because the write would land somewhere else. Nothing was written or touched. Replace the link with a plain folder or file and retry.",
      ),
      details,
      action: "report",
    };
  }

  // BetterFy's own map no longer matches its record. Either the install or the
  // restore found it, with or without the pak file also changed.
  const mapOnlyChanged =
    ownedMap !== null && ownedMap !== "intact" && (!owned || owned.state === "intact");
  if (code === "map_conflict" || (mapOnlyChanged && isFileCode(code))) {
    const restoring = code === "rollback_conflict";
    const gone = ownedMap === "missing";
    return {
      title: gone
        ? pick(isRu, "Карты BetterFy в игре больше нет", "BetterFy's map is gone from the game")
        : pick(isRu, "Карта BetterFy в игре изменилась", "BetterFy's map in the game changed"),
      body: gone
        ? pick(
            isRu,
            `Файл карты maps/dota.vpk пропал из ${where}, чаще всего после обновления Dota или проверки файлов в Steam. Восстанавливать нечего, ничего не повреждено. Нажми «Отпустить старую установку»: BetterFy забудет всю прежнюю установку, файлы в игре он не тронет. Потом поставь сборку заново.`,
            `The map file maps/dota.vpk is gone from ${where}, most often after a Dota update or a Steam file check. There is nothing to restore and nothing is damaged. Let go of the old install: BetterFy forgets the whole earlier installation and touches no game files. Then install the build again.`,
          )
        : pick(
            isRu,
            `Карту BetterFy maps/dota.vpk в ${where} изменили после установки, чаще всего это обновление Dota или проверка файлов в Steam. BetterFy не ${restoring ? "откатывает" : "затирает"} её старой копией, чтобы не отменить обновление игры. Нажми «Отпустить старую установку»: BetterFy забудет всю прежнюю установку, файлы в игре он не тронет, в том числе pak-файл, если он был.`,
            `BetterFy's map maps/dota.vpk in ${where} was changed after installation, most often by a Dota update or a Steam file check. BetterFy does not ${restoring ? "restore" : "overwrite"} it with an old copy, so a game update is not undone. Let go of the old install: BetterFy forgets the whole earlier installation and touches no game files, the pak file included if there was one.`,
          ),
      details,
      action: "release",
    };
  }

  if (code === "slot_full") {
    return {
      title: pick(isRu, "В папке языка нет места для файла", "No room for a file in the language folder"),
      body: pick(
        isRu,
        `${folder ? `В ${folder}` : "В папке языка"} заняты все номера pak50–pak65, которые использует BetterFy. Чужие файлы не тронуты, ставить новый файл некуда. Убери ненужные моды из этой папки или выбери другой язык.`,
        `${folder ? `${folder} has` : "The language folder has"} every number from pak50 to pak65 that BetterFy uses taken. Other files were not touched, and there is nowhere to put a new one. Remove unused mods from that folder or pick another language.`,
      ),
      details,
      action: null,
    };
  }

  if (
    code === "deployment_conflict" ||
    code === "rollback_conflict" ||
    code === "deployment_target_foreign"
  ) {
    const restoring = code === "rollback_conflict";
    const sameNameAsMinify =
      owned?.state === "replaced" &&
      report?.files.some((file) => file.name === owned.name && file.owner === "minify_like");
    if (owned && sameNameAsMinify) {
      const next = report?.nextName;
      return {
        title: pick(
          isRu,
          "Файл BetterFy заменила другая программа",
          "Another program replaced BetterFy's file",
        ),
        body: pick(
          isRu,
          `Файл BetterFy ${owned.name} в ${folder} заменила другая программа: то же имя использует Minify. BetterFy не тронул его и не стал ${restoring ? "откатывать поверх" : "затирать старой копией"}. Нажми «Отпустить старую установку»: файл Minify останется${next ? `, а BetterFy встанет рядом как ${next}` : ""}.`,
          `BetterFy's file ${owned.name} in ${folder} was replaced by another program; Minify writes the same file name. BetterFy left it alone and did not ${restoring ? "restore over it" : "overwrite it with an old copy"}. Let go of the old install: Minify's file stays${next ? ` and BetterFy installs next to it as ${next}` : ""}.`,
        ),
        details,
        action: "release",
      };
    }
    if (owned && owned.state === "missing") {
      return {
        title: pick(isRu, "Файла BetterFy в игре больше нет", "BetterFy's file is gone from the game"),
        body: pick(
          isRu,
          `Файл BetterFy ${owned.name} пропал из ${folder}, чаще всего после обновления Dota или проверки файлов в Steam. Восстанавливать нечего, ничего не повреждено. Нажми «Отпустить старую установку» и поставь сборку заново.`,
          `BetterFy's file ${owned.name} is gone from ${folder}, most often after a Dota update or a Steam file check. There is nothing to restore and nothing is damaged. Let go of the old install and install the build again.`,
        ),
        details,
        action: "release",
      };
    }
    if (owned && owned.state === "replaced") {
      return {
        title: pick(isRu, "Файл BetterFy в игре изменился", "BetterFy's file in the game changed"),
        body: pick(
          isRu,
          `Файл BetterFy ${owned.name} в ${folder} изменили после установки, чаще всего это обновление Dota или проверка файлов в Steam. BetterFy не ${restoring ? "откатывает" : "затирает"} его старой копией, чтобы не отменить обновление игры. Нажми «Отпустить старую установку»: файл игры не тронут.`,
          `BetterFy's file ${owned.name} in ${folder} was changed after installation, most often by a Dota update or a Steam file check. BetterFy does not ${restoring ? "restore" : "overwrite"} it with an old copy, so a game update is not undone. Let go of the old install: the game file is not touched.`,
        ),
        details,
        action: "release",
      };
    }
    // No usable facts: the same family of causes, said generally.
    return {
      title: pick(isRu, "Файл BetterFy в игре изменился", "BetterFy's file in the game changed"),
      body: pick(
        isRu,
        "Файл BetterFy в игре изменили или удалили после установки: это обновление Dota, проверка файлов в Steam или другая программа с тем же именем файла, например Minify. BetterFy не затирает его старой копией. «Отпустить старую установку» только забудет её, файлы игры не тронуты.",
        "BetterFy's file in the game was changed or removed after installation by a Dota update, a Steam file check, or another program using the same file name, such as Minify. BetterFy does not overwrite it with an old copy. Letting go of the old install only forgets it; no game files are touched.",
      ),
      details,
      action: code === "deployment_target_foreign" ? "report" : "release",
    };
  }

  if (code === "deployment_language_change_requires_restore") {
    return {
      title: pick(isRu, "Сначала откати установленную сборку", "Restore the installed build first"),
      body: pick(
        isRu,
        `Сборка BetterFy уже стоит${folder ? ` в ${folder}${owned ? ` (файл ${owned.name})` : ""}` : " в другом языке"}. Чтобы не оставить файлы сразу в двух языках, BetterFy не ставит новую поверх. Откати сборку, затем выбери другой язык.`,
        `A BetterFy build is already installed${folder ? ` in ${folder}${owned ? ` (file ${owned.name})` : ""}` : " in another language"}. So files are not left in two languages at once, BetterFy will not install another one. Restore the build, then pick another language.`,
      ),
      details,
      action: null,
    };
  }

  return null;
}
