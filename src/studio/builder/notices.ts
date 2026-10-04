// Every failure the builder can show, as a short title, the explanation and
// one recommended next step. The code itself stays under "Details" (and in
// the journal) so support can match it to the engine.

export type NoticeAction = "retry" | "prepare" | "settings" | "report" | "steam" | null;

export type Notice = {
  code: string;
  tone: "error" | "warn";
  title: string;
  body: string;
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
      "Откат не подтвердился проверкой. Сохрани отчёт и не меняй файлы вручную.",
      "The restore did not pass verification. Save the report and do not change files manually.",
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
    deployment_target_foreign: [
      "Целевой файл занят другой модификацией. BetterFy не будет его заменять.",
      "Another modification owns the target. BetterFy will not overwrite it.",
    ],
    deployment_language_change_requires_restore: [
      "Сначала откати установленную сборку, затем выбери другой язык.",
      "Restore the installed build before choosing another language.",
    ],
    language_folder_unavailable: [
      "В установке Dota нет папки выбранного языка с gameinfo.gi. Выбери другой язык или восстанови файлы игры через Steam.",
      "This Dota installation lacks the selected language folder with gameinfo.gi. Choose another language or verify game files in Steam.",
    ],
    launch_option_conflict: [
      "В Steam уже указан другой язык. Восстанови прежние параметры BetterFy или измени их вручную перед установкой.",
      "Steam already specifies a different language. Restore BetterFy's previous settings or change them manually before installing.",
    ],
    deployment_conflict: [
      "Файл игры изменился после установки. Автоматический откат остановлен.",
      "The game file changed after installation. Automatic restore was stopped.",
    ],
    backup_failed: [
      "Резервная копия BetterFy недоступна. Автоматический откат заблокирован; не заменяй файл вручную, пока не проверишь состояние установки.",
      "BetterFy's backup is unavailable. Automatic restore is blocked; inspect the installation before changing the file manually.",
    ],
    backup_verification_failed: [
      "Резервная копия не прошла проверку. BetterFy не будет восстанавливать её поверх файла игры.",
      "The backup failed verification. BetterFy will not restore it over the game file.",
    ],
    rollback_conflict: [
      "Файл игры изменился после установки. Автоматический откат остановлен.",
      "The game file changed after installation. Automatic restore was stopped.",
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
      "Найдено прерванное изменение Steam. Сначала восстанови его в диагностике.",
      "An interrupted Steam change was found. Recover it in diagnostics first.",
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
    codes: [
      "rollback_failed",
      "backup_failed",
      "backup_verification_failed",
      "deployment_conflict",
      "rollback_conflict",
      "deployment_target_foreign",
      "steam_recovery_required",
      "deployment_language_change_requires_restore",
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

export function noticeFor(code: string, isRu: boolean): Notice {
  const group = groups.find((item) => item.codes.includes(code));
  return {
    code,
    tone: group?.tone ?? "error",
    title: group ? group.title[isRu ? 0 : 1] : isRu ? "Операция остановлена" : "Operation stopped",
    body: explainError(code, isRu),
    action: group?.action ?? "report",
  };
}
