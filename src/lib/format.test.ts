import { describe, expect, it } from "vitest";

import { formatEta, formatSize, formatSpeed } from "./format";

describe("formatSize", () => {
  it("форматирует байты/КБ/МБ/ГБ", () => {
    expect(formatSize(0)).toBe("0 Б");
    expect(formatSize(512)).toBe("512 Б");
    expect(formatSize(1024)).toBe("1.00 КБ");
    expect(formatSize(700 * 1024 * 1024)).toBe("700 МБ");
    expect(formatSize(1.5 * 1024 ** 3)).toBe("1.50 ГБ");
  });

  it("не падает на отрицательных", () => {
    expect(formatSize(-1)).toBe("0 Б");
  });
});

describe("formatSpeed", () => {
  it("добавляет /с и прочерк для нуля", () => {
    expect(formatSpeed(0)).toBe("—");
    expect(formatSpeed(1024)).toBe("1.00 КБ/с");
  });
});

describe("formatEta", () => {
  it("форматирует интервалы", () => {
    expect(formatEta(null)).toBe("∞");
    expect(formatEta(0)).toBe("∞");
    expect(formatEta(45)).toBe("45с");
    expect(formatEta(125)).toBe("2м 5с");
    expect(formatEta(3700)).toBe("1ч 1м");
    expect(formatEta(90000)).toBe("1д 1ч");
  });
});
