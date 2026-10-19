// Every failure the builder can show, as a short title, the explanation and
// one recommended next step. The code itself stays under "Details" (and in
// the journal) so support can match it to the engine.

import type { LanguageSlotReport } from "../../engine";
import { explainSlot } from "./slot";

export type NoticeAction =
  | "retry"
  | "prepare"
  | "settings"
  | "report"
  | "steam"
  | "release"
  | null;

export type Notice = {
  code: string;
  tone: "error" | "warn";
  title: string;
  body: string;
  /** Facts for the "Details" fold, shown above the code. */
  details: string[];
  action: NoticeAction;
};

export const explainError = (code: string, isRu: boolean) => {
  const messages: Record<string, [string, string]> = {
    runtime_busy: ["Закрой Dota 2 и Steam, затем повтори.", "Close Dota 2 and Steam, then retry."],
    dota_close_unavailable: [
      "Не удалось мягко закрыть Dota 2. Закрой игру вручную и повтори.",
      "Dota 2 could not be closed gracefully. Close it manually and retry.",
    ],
    shutdown_timeout: [
      "Dota 2 или Steam не завершились за 30 секунд. Закрой их вручную и повтори.",
      "Dota 2 or Steam did not close within 30 seconds. Close them manually and retry.",
    ],
    steam_start_failed: [
      "Файл установлен, но Steam не запустился. Запусти Steam вручную и проверь параметры запуска.",
      "The file was installed, but Steam did not start. Open Steam manually and check launch options.",
    ],
    steam_start_timeout: [
      "Файл установлен, но BetterFy не дождался запуска Steam. Проверь Steam вручную.",
      "The file was installed, but BetterFy did not detect Steam starting. Check Steam manually.",
    ],
    platform_not_supported: [
      "Установка доступна только на Windows.",
      "Installation is Windows-only.",
    ],
    download_dns_failed: [
      "Нет доступа к источнику. Проверь подключение к сети.",
      "The source is unreachable. Check your connection.",
    ],
    download_transport_failed: [
      "Не удалось скачать ресурс. Повтори подготовку — проверенные файлы сохранятся.",
      "A resource could not be downloaded. Retry; verified files are kept.",
    ],
    package_conflict: [
      "Эти моды конфликтуют между собой. Оставь в сборке только один из них.",
      "These mods conflict. Keep only one of them in the build.",
    ],
    package_dependency_missing: [
      "Одному из модов нужен другой мод. Добавь его в сборку.",
      "One of the mods needs another mod. Add it to the build.",
    ],
    game_path_required: [
      "Для этой сборки нужна подключённая Dota 2: BetterFy сверяется с файлами твоей игры.",
      "This build needs a connected Dota 2: BetterFy checks it against your game files.",
    ],
    game_archive_unreadable: [
      "Не удалось прочитать архивы Dota 2. Проверь файлы игры в Steam и повтори.",
      "Dota 2 archives could not be read. Verify game files in Steam and retry.",
    ],
    blacklist_resolution_missing: [
      "Сборку нужно подготовить заново: список изменений для этой версии игры не найден.",
      "Prepare the build again: the change list for this game version is missing.",
    ],
    blacklist_resolution_stale: [
      "Сборку нужно подготовить заново: список изменений устарел.",
      "Prepare the build again: the change list is out of date.",
    ],
    steam_rollback_failed: [
      "Параметры запуска Steam вернуть не удалось. Проверь их в свойствах Dota 2 в Steam.",
      "Steam launch options could not be restored. Check them in Dota 2's properties in Steam.",
    ],
    steam_config_plan_stale: [
      "Steam изменил свои настройки во время операции. Закрой Steam и повтори.",
      "Steam changed its settings during the operation. Close Steam and retry.",
    ],
    steam_config_invalid: [
      "Файл настроек Steam не удалось прочитать. BetterFy не будет его менять.",
      "Steam's settings file could not be read. BetterFy will not change it.",
    ],
    rollback_failed: [
      "Откат не прошёл проверку: файл в игре не совпал с ожидаемым. BetterFy остановился на этом шаге и больше ничего не менял. Сохрани отчёт и не правь файлы вручную.",
      "The restore did not pass verification: the file in the game did not match what was expected. BetterFy stopped at that step and changed nothing else. Save the report and do not edit files by hand.",
    ],
    panorama_resolution_missing: [
      "Сборку нужно подготовить заново: снимок интерфейса игры не найден.",
      "Prepare the build again: the snapshot of the game interface is missing.",
    ],
    panorama_resolution_stale: [
      "Сборку нужно подготовить заново: снимок интерфейса игры устарел.",
      "Prepare the build again: the snapshot of the game interface is out of date.",
    ],
    panorama_layout_target_missing: [
      "Мод не подходит к текущей версии Dota 2: нужного элемента интерфейса больше нет. Убери мод из сборки.",
      "This mod does not fit the current Dota 2 version: the interface element it changes is gone. Remove it from the build.",
    ],
    panorama_layout_unsupported: [
      "Файл интерфейса этой версии Dota 2 в непривычном формате. BetterFy не будет его менять.",
      "This Dota 2 version stores the interface file in an unfamiliar format. BetterFy will not change it.",
    ],
    build_plan_stale: [
      "План устарел. Подготовь сборку заново.",
      "The plan is stale. Prepare the build again.",
    ],
    language_folder_unavailable: [
      "В установке Dota нет папки выбранного языка с gameinfo.gi. Выбери другой язык или восстанови файлы игры через Steam.",
      "This Dota installation lacks the selected language folder with gameinfo.gi. Choose another language or verify game files in Steam.",
    ],
    launch_option_conflict: [
      "В Steam уже указан другой язык. Восстанови прежние параметры BetterFy или измени их вручную перед установкой.",
      "Steam already specifies a different language. Restore BetterFy's previous settings or change them manually before installing.",
    ],
    backup_failed: [
      "BetterFy не смог прочитать или записать свою резервную копию. Поэтому он остановился: автоматический откат заблокирован, чтобы не записать в игру непроверенные данные. Сохрани отчёт и не правь файл вручную.",
      "BetterFy could not read or write its backup. It stopped, and automatic restore is blocked so unverified data is not written into the game. Save the report and do not edit the file by hand.",
    ],
    backup_verification_failed: [
      "Резервная копия не прошла проверку: её содержимое не совпало с тем, что было записано. BetterFy не записывает и не восстанавливает файл, пока не уверен в копии. Сохрани отчёт; файл игры не тронут.",
      "The backup failed verification: its contents did not match what was recorded. BetterFy will not write or restore a file until it can trust the backup. Save the report; the game file was not touched.",
    ],
    deployment_not_stale: [
      "Установка BetterFy на месте и не изменилась, поэтому отпускать её не нужно. Файлы игры не тронуты. Если что-то не так, используй откат.",
      "The BetterFy install is intact and unchanged, so there is nothing to let go of. No game files were touched. Use restore if something is off.",
    ],
    steam_profile_conflict: [
      "Этот Steam-профиль нельзя менять автоматически. Выбери другой или проверь параметры запуска вручную.",
      "This Steam profile cannot be changed automatically. Choose another or inspect its launch options.",
    ],
    steam_activation_not_ready: [
      "Параметры Steam не подтверждены. Не считай мод активным в игре.",
      "Steam settings were not verified. Do not assume the mod is active in game.",
    ],
    steam_recovery_required: [
      "Прошлое изменение параметров запуска Steam прервалось и не закончено. Пока оно висит, BetterFy не трогает ни файл игры, ни Steam, чтобы не оставить игру с неверным языком. Сначала восстанови его в диагностике, потом повтори.",
      "An earlier change to Steam's launch options was interrupted and is unfinished. Until it is resolved BetterFy leaves both the game file and Steam alone, so the game is not left with a wrong language. Recover it in diagnostics first, then retry.",
    ],
    clipboard_unavailable: [
      "Не удалось скопировать параметр. Выдели и скопируй его вручную.",
      "Could not copy the option. Select and copy it manually.",
    ],
    stress_requires_unmanaged_profile: [
      "Для этого теста выбери Steam-профиль, где BetterFy ещё не добавлял выбранный язык.",
      "Choose a Steam profile where BetterFy has not already added the selected language.",
    ],
    stress_recovery_unverified: [
      "Тестовое восстановление не дало ожидаемого подтверждения. Не запускай Dota и скопируй отчёт.",
      "Test recovery did not produce the expected proof. Do not launch Dota; copy the report.",
    ],
    stress_injection_did_not_fire: [
      "Тестовая точка отказа не сработала. Обычная установка не подтверждена этим тестом.",
      "The test failure point did not fire. This test did not verify the normal installation.",
    ],
    stress_test_disabled: [
      "Контролируемый стресс-тест доступен только аккаунту разработчика.",
      "Controlled stress testing is available only to a developer account.",
    ],
    wardrobe_conflict_unacknowledged: [
      "В сборке есть файлы, которые меняют сразу несколько наборов. Просмотри список и подтверди его, затем повтори.",
      "Some files are changed by more than one item in the build. Review the list, confirm it, and retry.",
    ],
    bundle_limit_exceeded: [
      "Сборка слишком большая: в одну сборку помещается до 512 МБ. Ничего не скачано. Убери один из крупных обликов или пакетов (музыка, деревья, курьеры с моделями) или, для облика героя, включи «Менять только этого героя» (так сборка меньше), затем подготовь заново.",
      "The build is too large: one build holds up to 512 MB. Nothing was downloaded. Remove one of the large skins or packs (music, trees, courier models) or, for a hero skin, turn on \"Change only this hero\" (it makes the build smaller), then prepare again.",
    ],
    wardrobe_audit_mismatch: [
      "Архив не совпал с проверенным описанием. BetterFy не будет его ставить.",
      "The archive does not match its audited description. BetterFy will not install it.",
    ],
    wardrobe_archive_hash_mismatch: [
      "Архив скачался с ошибкой, BetterFy его не сохранил. Попробуй ещё раз.",
      "The archive downloaded with an error and BetterFy did not keep it. Try again.",
    ],
    wardrobe_archive_unexpected: [
      "Внутри архива не то, что ожидалось. BetterFy не будет его ставить.",
      "The archive does not contain what was expected. BetterFy will not install it.",
    ],
    skin_archive_hostile: [
      "В архиве есть файлы, которые BetterFy не ставит ни при каких условиях (скрипты или программы).",
      "The archive holds files BetterFy never installs (scripts or programs).",
    ],
    skin_shared_paths_refused: [
      "Архив меняет общие файлы игры, а для него выбрано «отказать». Установка остановлена.",
      "The archive changes shared game files and is set to refuse them. Installation stopped.",
    ],
    build_too_large: [
      "Всё выбранное вместе больше 512 МБ, а это предел одной сборки. Убери один из крупных обликов или собери его отдельно от остального.",
      "Everything selected adds up to more than 512 MB, the limit for one build. Remove one of the large skins or build it separately from the rest.",
    ],
    skin_install_too_large: [
      "Набор слишком большой для одной сборки.",
      "This item is too large for one build.",
    ],
    map_target_foreign: [
      "В папке языка уже лежит файл maps/dota.vpk, который записал не BetterFy. Он его не тронул и ничего не записал. Убери карту из сборки или перенеси этот файл сам и повтори.",
      "The language folder already has a maps/dota.vpk that BetterFy did not write. It left it alone and wrote nothing. Remove the map from the build, or move that file yourself and retry.",
    ],
    map_conflict: [
      "Карта BetterFy в игре изменилась или пропала после установки. BetterFy не затирает её старой копией. Отпусти старую установку и поставь сборку заново.",
      "BetterFy's map in the game was changed or removed after installation. BetterFy does not overwrite it with an old copy. Let go of the old install and install the build again.",
    ],
    map_folder_invalid: [
      "Под именем maps в папке языка лежит не обычная папка. BetterFy ничего не записал. Убери или переименуй это и повтори.",
      "Something named maps in the language folder is not a plain folder. BetterFy wrote nothing. Remove or rename it and retry.",
    ],
    deployment_path_unsafe: [
      "Путь записи проходит через ссылку (symlink или junction). BetterFy не пишет через ссылки. Ничего не записано.",
      "The write path goes through a link (a symlink or a junction). BetterFy does not write through links. Nothing was written.",
    ],
    map_terrain_conflict: [
      "В сборке две карты, а ставится только одна: карта заменяет файл карты целиком. Оставь одну из них, остальное в сборке не изменится.",
      "The build has two maps, and only one can be installed: a map replaces the game's map file as a whole. Keep one of them; the rest of the build stays as is.",
    ],
    user_styles_selection_required: [
      "Для «Пользовательских стилей» не выбран ни один стиль. Открой карточку мода и отметь нужные.",
      "No style is picked for User Styles. Open the mod's card and tick the ones you want.",
    ],
    game_pack_conflict: [
      "В сборке два курсора или два набора шрифтов, а ставится только один каждого вида. Оставь один, остальное в сборке не изменится.",
      "The build has two cursors or two font packs, and only one of each can be installed. Keep one; the rest of the build stays as is.",
    ],
    game_pack_hash_mismatch: [
      "Архив курсора или шрифтов скачался с ошибкой, BetterFy его не сохранил. Попробуй ещё раз.",
      "The cursor or font archive downloaded with an error and BetterFy did not keep it. Try again.",
    ],
    game_pack_unexpected: [
      "Внутри архива курсора или шрифтов не то, что ожидалось. BetterFy не будет его ставить.",
      "The cursor or font archive does not contain what was expected. BetterFy will not install it.",
    ],
    game_pack_content_invalid: [
      "Файл курсора или шрифта не похож на настоящий. BetterFy не будет его ставить.",
      "A cursor or font file does not look like a real one. BetterFy will not install it.",
    ],
    game_file_folder_missing: [
      "В Dota нет папки курсоров, шрифтов или гайдов. Проверь файлы игры в Steam и повтори. Ничего не записано.",
      "Dota is missing its cursor, font or guide folder. Verify the game files in Steam and retry. Nothing was written.",
    ],
    game_file_target_unsafe: [
      "На месте файла курсора или шрифта в Dota лежит что-то другое (папка или ссылка). BetterFy ничего не записал.",
      "Something other than a file sits where a Dota cursor or font file belongs (a folder or a link). BetterFy wrote nothing.",
    ],
    game_files_recovery_required: [
      "Прошлая запись курсоров или шрифтов оборвалась. Нажми «Проверить» в подробностях — BetterFy вернёт файлы — и повтори.",
      "An earlier write of cursors or fonts stopped half way. Use Check under Details so BetterFy puts the files back, then retry.",
    ],
    game_files_not_latest: [
      "Откатить можно только последнюю сборку. Сначала откати её.",
      "Only the latest build can be restored. Restore that one first.",
    ],
    game_file_write_failed: [
      "Не удалось записать файл курсора или шрифта. Закрой Dota и программы, которые могут его держать, и повтори. Сборка отменена.",
      "A cursor or font file could not be written. Close Dota and anything that may hold it, and retry. The build was undone.",
    ],
    map_audit_mismatch: [
      "Карта не совпала с проверенным описанием: внутри не то, что записано при аудите. BetterFy не будет её ставить.",
      "The map does not match its audited description: what is inside is not what the audit recorded. BetterFy will not install it.",
    ],
    map_archive_hash_mismatch: [
      "Архив карты скачался с ошибкой, BetterFy его не сохранил. Попробуй ещё раз.",
      "The map archive downloaded with an error and BetterFy did not keep it. Try again.",
    ],
    map_hash_mismatch: [
      "Файл карты повреждён, поэтому ничего не записано. Подготовь сборку заново.",
      "The map file is damaged, so nothing was written. Prepare the build again.",
    ],
    map_archive_unexpected: [
      "Внутри архива карты не то, что ожидалось (нужны только maps/dota.vpk и Guide.txt). BetterFy не будет его ставить.",
      "The map archive does not contain what was expected (only maps/dota.vpk and Guide.txt). BetterFy will not install it.",
    ],
    map_archive_unreadable: [
      "Архив карты не удалось прочитать. Подготовь сборку заново.",
      "The map archive could not be read. Prepare the build again.",
    ],
    map_package_invalid: [
      "Файл карты не прошёл проверку формата: это не целый и корректный пакет карты. BetterFy не будет его ставить.",
      "The map file failed the format check: it is not a complete, well-formed map package. BetterFy will not install it.",
    ],
    map_package_unsupported: [
      "Пакет карты в формате, который BetterFy не читает (части в других файлах, подпись или неизвестная версия). Он не будет его ставить.",
      "The map package is in a format BetterFy does not read (parts in other files, a signature, or an unknown version). It will not install it.",
    ],
    map_entry_script: [
      "В пакете карты есть скрипт или программа. BetterFy такое не ставит ни при каких условиях.",
      "The map package holds a script or a program. BetterFy never installs that.",
    ],
    map_entry_type_blocked: [
      "В пакете карты есть файл типа, который не входит в проверенный набор для карт. BetterFy не будет его ставить.",
      "The map package holds a file of a type outside the audited set for maps. BetterFy will not install it.",
    ],
    map_entry_path_invalid: [
      "В пакете карты есть файл с небезопасным или некорректным именем. BetterFy не будет его ставить.",
      "The map package holds a file with an unsafe or malformed name. BetterFy will not install it.",
    ],
    map_entry_outside_maps: [
      "В пакете карты есть файлы вне папки maps: карта не должна заменять ничего другого. BetterFy не будет её ставить.",
      "The map package holds files outside the maps folder: a map must not replace anything else. BetterFy will not install it.",
    ],
    map_entry_duplicate: [
      "В пакете карты один и тот же путь указан дважды. BetterFy не будет его ставить.",
      "The map package lists the same path twice. BetterFy will not install it.",
    ],
    map_main_missing: [
      "В пакете нет основного файла карты (maps/dota.vmap_c). BetterFy не будет его ставить.",
      "The package has no main map file (maps/dota.vmap_c). BetterFy will not install it.",
    ],
  };
  return (
    messages[code]?.[isRu ? 0 : 1] ??
    (isRu
      ? `Операция остановлена (${code}). Проверь состояние установки перед запуском игры.`
      : `Operation stopped (${code}). Check installation state before launching the game.`)
  );
};

const groups: Array<{
  codes: string[];
  title: [string, string];
  action: NoticeAction;
  tone?: "error" | "warn";
}> = [
  {
    codes: ["runtime_busy", "dota_close_unavailable", "shutdown_timeout"],
    title: ["Dota или Steam ещё открыты", "Dota or Steam is still open"],
    action: "retry",
    tone: "warn",
  },
  {
    codes: ["download_dns_failed", "download_transport_failed", "download_failed"],
    title: ["Не удалось скачать файлы модов", "Mod files could not be downloaded"],
    action: "prepare",
    tone: "warn",
  },
  {
    codes: ["package_conflict", "package_dependency_missing"],
    title: ["Моды не уживаются вместе", "These mods do not fit together"],
    action: null,
    tone: "warn",
  },
  {
    codes: ["map_terrain_conflict"],
    title: ["В сборке две карты", "Two maps in one build"],
    action: null,
    tone: "warn",
  },
  {
    codes: ["game_pack_conflict"],
    title: ["Два курсора или два набора шрифтов", "Two cursors or two font packs"],
    action: null,
    tone: "warn",
  },
  {
    codes: ["wardrobe_conflict_unacknowledged"],
    title: ["Нужно подтвердить общие файлы", "Shared files need your confirmation"],
    action: null,
    tone: "warn",
  },
  {
    codes: ["bundle_limit_exceeded", "build_too_large"],
    title: ["Сборка слишком большая", "The build is too big"],
    action: null,
    tone: "warn",
  },
  {
    codes: [
      "wardrobe_audit_mismatch",
      "wardrobe_archive_hash_mismatch",
      "wardrobe_archive_unexpected",
      "skin_archive_hostile",
      "skin_shared_paths_refused",
      "skin_install_too_large",
      "map_audit_mismatch",
      "map_archive_hash_mismatch",
      "map_hash_mismatch",
      "map_archive_unexpected",
      "map_archive_unreadable",
      "map_package_invalid",
      "map_package_unsupported",
      "map_entry_script",
      "map_entry_type_blocked",
      "map_entry_path_invalid",
      "map_entry_outside_maps",
      "map_entry_duplicate",
      "map_main_missing",
    ],
    title: ["Набор не прошёл проверку", "This item did not pass the check"],
    action: null,
  },
  {
    codes: ["game_path_required", "language_folder_unavailable", "game_archive_unreadable"],
    title: ["Нужна проверка Dota 2", "Dota 2 needs a check"],
    action: "settings",
  },
  {
    codes: [
      "blacklist_resolution_missing",
      "blacklist_resolution_stale",
      "panorama_resolution_missing",
      "panorama_resolution_stale",
      "build_plan_stale",
    ],
    title: ["Сборку нужно подготовить заново", "Prepare the build again"],
    action: "prepare",
    tone: "warn",
  },
  {
    codes: ["panorama_layout_target_missing", "panorama_layout_unsupported"],
    title: ["Мод не подходит к этой версии Dota", "A mod does not fit this Dota version"],
    action: null,
  },
  {
    codes: [
      "steam_start_failed",
      "steam_start_timeout",
      "steam_rollback_failed",
      "steam_config_plan_stale",
      "steam_config_invalid",
      "steam_profile_conflict",
      "steam_activation_not_ready",
      "launch_option_conflict",
    ],
    title: ["Steam не настроен до конца", "Steam setup did not finish"],
    action: "steam",
    tone: "warn",
  },
  {
    // Wording and action come from the folder's actual state (slot.ts).
    codes: [
      "deployment_conflict",
      "rollback_conflict",
      "deployment_target_foreign",
      "deployment_language_change_requires_restore",
      "slot_full",
      "map_conflict",
    ],
    title: ["Файл игры изменился", "The game file changed"],
    action: "report",
    tone: "warn",
  },
  {
    // Wording and action come from the folder's actual state (slot.ts).
    codes: ["map_target_foreign", "map_folder_invalid"],
    title: ["В папке языка уже есть файл карты", "The language folder already has a map file"],
    action: "retry",
    tone: "warn",
  },
  {
    codes: ["deployment_path_unsafe"],
    title: ["Путь записи — это ссылка", "The write path is a link"],
    action: "report",
    tone: "warn",
  },
  {
    codes: [
      "rollback_failed",
      "backup_failed",
      "backup_verification_failed",
      "steam_recovery_required",
    ],
    title: ["BetterFy остановился, чтобы ничего не сломать", "BetterFy stopped so nothing breaks"],
    action: "report",
  },
  {
    codes: ["platform_not_supported"],
    title: ["Нужен Windows", "Windows is required"],
    action: null,
    tone: "warn",
  },
  {
    codes: ["clipboard_unavailable"],
    title: ["Не удалось скопировать", "Could not copy"],
    action: null,
    tone: "warn",
  },
  {
    codes: [
      "stress_requires_unmanaged_profile",
      "stress_recovery_unverified",
      "stress_injection_did_not_fire",
      "stress_test_disabled",
    ],
    title: ["Стресс-тест не прошёл", "Stress test did not pass"],
    action: "report",
  },
];

/** The refusals that depend on what sits in the language folder. */
export const slotCodes = [
  "deployment_conflict",
  "rollback_conflict",
  "deployment_target_foreign",
  "deployment_language_change_requires_restore",
  "slot_full",
  "map_target_foreign",
  "map_conflict",
  "map_folder_invalid",
  "deployment_path_unsafe",
];

/**
 * `slot` is the read-only summary of the language folder, when it has been
 * read: it turns the general wording into the actual cause.
 */
export function noticeFor(
  code: string,
  isRu: boolean,
  slot: LanguageSlotReport | null = null,
): Notice {
  const group = groups.find((item) => item.codes.includes(code));
  const slotText = explainSlot(code, isRu, slot);
  if (slotText) {
    return {
      code,
      tone: "warn",
      title: slotText.title,
      body: slotText.body,
      details: slotText.details,
      action: slotText.action,
    };
  }
  return {
    code,
    tone: group?.tone ?? "error",
    title: group ? group.title[isRu ? 0 : 1] : isRu ? "Операция остановлена" : "Operation stopped",
    body: explainError(code, isRu),
    details: [],
    action: group?.action ?? "report",
  };
}
