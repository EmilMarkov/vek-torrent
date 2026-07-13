import { describe, expect, it } from "vitest";

import {
  applyFilters,
  DEFAULT_FILTERS,
  effectiveCategoryForumIds,
  forumIdsForCategory,
  GENERAL_CATEGORIES,
  parseRefine,
  parseSizeInput,
} from "./filters";
import type { ForumGroup, SearchResult } from "./types";

function result(over: Partial<SearchResult>): SearchResult {
  return {
    topic_id: 1,
    title: "Образец раздачи",
    forum: { id: 10, name: "Форум" },
    author: "автор",
    size_bytes: 1024,
    seeders: 5,
    leechers: 1,
    downloads: 100,
    added_unix: 1000,
    approval: "approved",
    ...over,
  };
}

describe("parseRefine", () => {
  const word = (value: string) => ({ value, phrase: false });
  const quote = (value: string) => ({ value, phrase: true });

  it("делит на включающие и исключающие термины", () => {
    const parsed = parseRefine("linux mint -beta -rc");
    expect(parsed.include).toEqual([word("linux"), word("mint")]);
    expect(parsed.exclude).toEqual([word("beta"), word("rc")]);
  });

  it("игнорирует лишние пробелы и одинокий минус", () => {
    const parsed = parseRefine("  ubuntu   -  ");
    expect(parsed.include).toEqual([word("ubuntu")]);
    expect(parsed.exclude).toEqual([]);
  });

  it("поддерживает словосочетания в кавычках, включая исключающие", () => {
    const parsed = parseRefine('linux "linux mint" -beta -"release candidate"');
    expect(parsed.include).toEqual([word("linux"), quote("linux mint")]);
    expect(parsed.exclude).toEqual([word("beta"), quote("release candidate")]);
  });

  it('исключает словосочетание -"Fallout Shelter"', () => {
    const parsed = parseRefine('fallout -"fallout shelter"');
    expect(parsed.include).toEqual([word("fallout")]);
    expect(parsed.exclude).toEqual([quote("fallout shelter")]);
  });

  it("срезает окаймляющую пунктуацию у слов, но не у фраз", () => {
    expect(parseRefine("сезон: 3").include).toEqual([word("сезон"), word("3")]);
    expect(parseRefine('"сезон: 3"').include).toEqual([quote("сезон: 3")]);
  });
});

describe("applyFilters", () => {
  const items = [
    result({ topic_id: 1, title: "Linux Mint 21.3", size_bytes: 2_000_000_000, seeders: 100 }),
    result({ topic_id: 2, title: "Ubuntu 24.04 beta", size_bytes: 3_000_000_000, seeders: 50 }),
    result({
      topic_id: 3,
      title: "Debian 12",
      size_bytes: 500_000_000,
      seeders: 5,
      approval: "not_approved",
      forum: { id: 20, name: "Другой" },
    }),
  ];

  it("без фильтров возвращает всё", () => {
    expect(applyFilters(items, DEFAULT_FILTERS)).toHaveLength(3);
  });

  it("фильтрует по включающему термину", () => {
    const out = applyFilters(items, { ...DEFAULT_FILTERS, refine: "linux" });
    expect(out).toHaveLength(1);
    expect(out[0].topic_id).toBe(1);
  });

  it("исключает по префиксу минус", () => {
    const out = applyFilters(items, { ...DEFAULT_FILTERS, refine: "-beta" });
    expect(out.map((r) => r.topic_id)).toEqual([1, 3]);
  });

  it("фильтрует по минимальному размеру и сидам", () => {
    const out = applyFilters(items, {
      ...DEFAULT_FILTERS,
      minSizeBytes: 1_000_000_000,
      minSeeders: 60,
    });
    expect(out.map((r) => r.topic_id)).toEqual([1]);
  });

  it("только проверенные", () => {
    const out = applyFilters(items, { ...DEFAULT_FILTERS, onlyApproved: true });
    expect(out.every((r) => r.approval === "approved")).toBe(true);
    expect(out).toHaveLength(2);
  });

  it("сохраняет серверный порядок результатов", () => {
    // Сортировка — серверная; фильтры не должны переупорядочивать выдачу.
    const out = applyFilters(items, { ...DEFAULT_FILTERS, refine: "-beta" });
    expect(out.map((r) => r.topic_id)).toEqual([1, 3]);
  });

  it("не мутирует исходный массив", () => {
    const snapshot = items.map((r) => r.topic_id);
    applyFilters(items, { ...DEFAULT_FILTERS, refine: "linux" });
    expect(items.map((r) => r.topic_id)).toEqual(snapshot);
  });

  it("исключает словосочетание из выдачи", () => {
    const out = applyFilters(items, { ...DEFAULT_FILTERS, refine: '-"ubuntu 24.04"' });
    expect(out.map((r) => r.topic_id)).toEqual([1, 3]);
  });

  it("слово сопоставляется по границам токена: «3» не ловит «15K3»", () => {
    const seasons = [
      result({ topic_id: 10, title: "Дом дракона / Сезон: 3 / Серии: 1-3 из 8 [2026, WEB-DL]" }),
      result({
        topic_id: 11,
        title: "Дом дракона / Сезон: 2 / Серии: 1-8 из 8 [2024] 2 x MVO (Syncmer, 15K3)",
      }),
    ];
    const out = applyFilters(seasons, { ...DEFAULT_FILTERS, refine: "сезон: 3" });
    expect(out.map((r) => r.topic_id)).toEqual([10]);
  });

  it("уточнение «сезон 3» находит запись с «Сезон: 3» несмотря на двоеточие", () => {
    const seasons = [result({ topic_id: 12, title: "Сериал / Сезон: 3 / серии 1-10" })];
    const out = applyFilters(seasons, { ...DEFAULT_FILTERS, refine: "сезон 3" });
    expect(out.map((r) => r.topic_id)).toEqual([12]);
  });

  it("префиксное слово находит разрешение: «1080» ловит «1080p»", () => {
    const res = [
      result({ topic_id: 20, title: "Фильм [WEB-DL 1080p]" }),
      result({ topic_id: 21, title: "Фильм [WEB-DL 720p]" }),
    ];
    const out = applyFilters(res, { ...DEFAULT_FILTERS, refine: "1080" });
    expect(out.map((r) => r.topic_id)).toEqual([20]);
  });

  it("автор и разделы не фильтруются на клиенте (уходят на сервер)", () => {
    const out = applyFilters(items, {
      ...DEFAULT_FILTERS,
      author: "кто-то",
      forumIds: [999],
      categoryIds: ["x"],
    });
    expect(out).toHaveLength(3);
  });
});

describe("forumIdsForCategory", () => {
  const groups: ForumGroup[] = [
    { title: "Кино, Видео и ТВ", forums: [{ id: 1, name: "Зарубежное кино", depth: 0 }] },
    { title: "Игры", forums: [{ id: 2, name: "Игры для PC", depth: 0 }] },
    { title: "Музыка", forums: [{ id: 3, name: "Поп-музыка", depth: 0 }] },
    { title: "Книги и журналы", forums: [{ id: 4, name: "Аудиокниги", depth: 0 }] },
  ];

  const cat = (key: string) => GENERAL_CATEGORIES.find((c) => c.key === key)!;

  it("маппит разделы по ключевым словам", () => {
    expect(forumIdsForCategory(groups, cat("films"))).toContain(1);
    expect(forumIdsForCategory(groups, cat("games"))).toContain(2);
    expect(forumIdsForCategory(groups, cat("music"))).toContain(3);
    expect(forumIdsForCategory(groups, cat("books"))).toContain(4);
  });

  it("аудиокниги относятся к книгам, а не к музыке", () => {
    expect(forumIdsForCategory(groups, cat("music"))).not.toContain(4);
    expect(forumIdsForCategory(groups, cat("books"))).toContain(4);
  });
});

describe("effectiveCategoryForumIds", () => {
  const groups: ForumGroup[] = [
    { title: "Игры", forums: [{ id: 2, name: "Игры для PC", depth: 0 }] },
  ];

  it("явно заданные разделы имеют приоритет", () => {
    const category = { name: "Игры", forumIds: [7, 8] };
    expect(effectiveCategoryForumIds(groups, category)).toEqual([7, 8]);
  });

  it("стандартная категория без настройки использует эвристику по имени", () => {
    const category = { name: "игры", forumIds: [] };
    expect(effectiveCategoryForumIds(groups, category)).toEqual([2]);
  });

  it("своя категория без разделов даёт пустой набор", () => {
    const category = { name: "Аниме", forumIds: [] };
    expect(effectiveCategoryForumIds(groups, category)).toEqual([]);
  });
});

describe("parseSizeInput", () => {
  it("парсит единицы измерения", () => {
    expect(parseSizeInput("1.5 гб")).toBe(Math.round(1.5 * 1024 ** 3));
    expect(parseSizeInput("700мб")).toBe(700 * 1024 ** 2);
    expect(parseSizeInput("2 GB")).toBe(2 * 1024 ** 3);
  });

  it("по умолчанию мегабайты", () => {
    expect(parseSizeInput("500")).toBe(500 * 1024 ** 2);
  });

  it("возвращает null на мусоре и пустоте", () => {
    expect(parseSizeInput("")).toBeNull();
    expect(parseSizeInput("abc")).toBeNull();
  });
});
