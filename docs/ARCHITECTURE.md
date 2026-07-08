# VEK Torrent — архитектура и план разработки

Внутренний документ для разработки и код-ревью. Пользовательская документация — `README.md`.

## Обзор

Десктоп-приложение (Tauri 2 + React): поиск и просмотр раздач rutracker с улучшенным UX,
встроенный менеджер загрузок поверх qBittorrent (sidecar), внешний REST API для интеграций.

## Модули

Cargo workspace, зависимости направлены строго вниз:
`src-tauri` → `vek-api` → `vek-core` → { `rutracker`, `qbit` }.
Фронтенд общается только через Tauri IPC; внешний API — отдельный вход в те же сервисы.

- **`crates/rutracker`** — клиент rutracker: сессия (cookies, персистентность), логин (в т.ч.
  капча), зеркала и прокси, поиск (`tracker.php`), страница раздачи (`viewtopic.php`) с разбором
  контента в структурную блочную модель, дерево категорий, скачивание `.torrent` (`dl.php`).
  Кодировка windows-1251 → UTF-8 (`encoding_rs`). Парсинг — `scraper`, устойчивый к мелким
  изменениям разметки (без паник, деградация с пропуском полей).
- **`crates/qbit`** — типизированный клиент qBittorrent Web API v2 (цель — qBittorrent 5.x,
  fallback 4.x: `stop/start` ↔ `pause/resume`): torrents info/add/delete, `sync/maindata` (rid),
  transfer info, app version/shutdown, categories.
- **`crates/vek-core`** — доменный слой: конфиг (JSON в app-dir), сервисы `SearchService`,
  `TopicService`, `DownloadsService`, sidecar-менеджер qBittorrent (автопоиск бинарника,
  изолированный профиль, свободный порт, health-check, graceful shutdown, watchdog),
  единый `Error`.
- **`crates/vek-api`** — внешний REST API (axum): `/api/v1` (search, topics, downloads,
  categories, transfer), Bearer-токен, OpenAPI (utoipa) + Swagger UI на `/docs`,
  bind `127.0.0.1` (настраивается).
- **`src-tauri`** — приложение: состояние (`Arc<AppCore>`), тонкие команды над core, события,
  запуск/перезапуск API-сервера, жизненный цикл sidecar.
- **`src`** — React + TypeScript + Vite + Tailwind CSS: тёмная тема, страницы
  Поиск / Раздача / Загрузки / Настройки. Серверное состояние — TanStack Query,
  клиентское — Zustand.

## Ключевые решения

- **qBittorrent только sidecar**: приложение само запускает установленный
  `qbittorrent-nox`/`qbittorrent` с профилем в app-data и WebUI на `127.0.0.1:<свободный порт>`,
  аутентификация для localhost отключена в генерируемом профиле. Автопоиск бинарника
  (PATH + стандартные пути), путь настраивается в UI.
- **Поиск**: серверные параметры rutracker (форумы, сортировка, автор, пагинация `start=N`) +
  мгновенные клиентские фильтры по уже загруженным результатам (текст вкл/искл, размер, сиды,
  категории, только проверенные) без повторных запросов. Lazy-search: debounce + отмена
  устаревших запросов + infinite scroll.
- **Страница раздачи**: `post_body` парсится в JSON-блоки (параграфы, спойлеры, цитаты,
  изображения, код, списки, ссылки, форматирование) — рендер собственным UI, чужой HTML
  в webview не вставляется.
- **Капча логина**: сервис возвращает `NeedsCaptcha { image, sid, code_field }` → UI показывает
  картинку → повторный логин с кодом. Сессионные куки кешируются, авто-релогин при протухании.
- **Секреты**: учётные данные в локальном конфиге приложения; наружу (API/логи) не отдаются.

## Тестирование

- Парсеры rutracker: unit-тесты на HTML-фикстурах (включая windows-1251).
- Клиенты rutracker/qbit: интеграционные тесты через wiremock.
- vek-api: тесты хендлеров (tower `oneshot`) + проверка авторизации.
- Frontend: vitest + Testing Library (клиентские фильтры, рендер блоков, формы).
- CI: `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace`, eslint, `tsc`,
  vitest, `vite build`.

## CI/CD

- `ci.yml` — линт + тесты на push/PR.
- `Cargo.lock` коммитится после первого прогона CI (в среде разработки cargo недоступен);
  после этого сборки в CI переводятся на `--locked`.
- `release.yml` — на тег `v*`: создание GitHub-релиза с автогенерируемыми release notes,
  сборка через tauri-action: Windows (NSIS/MSI), Linux (AppImage/deb/rpm), macOS
  (dmg, aarch64 + x86_64).

## Этапы

После каждого этапа — агентное код-ревью и исправление замечаний.

1. Скелет: workspace, Tauri 2, React + Vite + Tailwind, CI-каркас.
2. `crates/rutracker` + тесты.
3. `crates/qbit` + sidecar + `vek-core`.
4. `vek-api` + Tauri-команды.
5. Frontend полностью.
6. Release-workflow, README, иконки, финальная верификация.
