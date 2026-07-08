import { describe, expect, it } from "vitest";

import { applyFilters, DEFAULT_FILTERS, parseRefine, parseSizeInput } from "./filters";
import type { SearchResult } from "./types";

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
  it("делит на включающие и исключающие термины", () => {
    const parsed = parseRefine("linux mint -beta -rc");
    expect(parsed.include).toEqual(["linux", "mint"]);
    expect(parsed.exclude).toEqual(["beta", "rc"]);
  });

  it("игнорирует лишние пробелы и одинокий минус", () => {
    const parsed = parseRefine("  ubuntu   -  ");
    expect(parsed.include).toEqual(["ubuntu"]);
    expect(parsed.exclude).toEqual([]);
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

  it("ограничивает по форуму", () => {
    const out = applyFilters(items, { ...DEFAULT_FILTERS, forumIds: [20] });
    expect(out.map((r) => r.topic_id)).toEqual([3]);
  });

  it("сортирует по сидам по убыванию", () => {
    const out = applyFilters(items, { ...DEFAULT_FILTERS, sortKey: "seeders", sortDesc: true });
    expect(out.map((r) => r.seeders)).toEqual([100, 50, 5]);
  });

  it("сортирует по размеру по возрастанию", () => {
    const out = applyFilters(items, { ...DEFAULT_FILTERS, sortKey: "size", sortDesc: false });
    expect(out.map((r) => r.topic_id)).toEqual([3, 1, 2]);
  });

  it("не мутирует исходный массив", () => {
    const snapshot = items.map((r) => r.topic_id);
    applyFilters(items, { ...DEFAULT_FILTERS, sortKey: "seeders", sortDesc: true });
    expect(items.map((r) => r.topic_id)).toEqual(snapshot);
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
