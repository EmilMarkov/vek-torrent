import { describe, expect, it } from "vitest";

import { adaptColorForDark } from "./colors";

describe("adaptColorForDark", () => {
  it("почти чёрный превращает в цвет по умолчанию", () => {
    expect(adaptColorForDark("black")).toBeUndefined();
    expect(adaptColorForDark("#000")).toBeUndefined();
    expect(adaptColorForDark("#0a0a0a")).toBeUndefined();
  });

  it("тёмные цвета осветляет, сохраняя тон", () => {
    const navy = adaptColorForDark("darkblue");
    expect(navy).toMatch(/^hsl\(240, /); // тот же синий тон
    expect(navy).toMatch(/6[0-9]%\)$/); // светлота поднята

    expect(adaptColorForDark("#8b0000")).toMatch(/^hsl\(0, /); // darkred
  });

  it("светлые цвета не трогает", () => {
    expect(adaptColorForDark("yellow")).toBe("yellow");
    expect(adaptColorForDark("#87ceeb")).toBe("#87ceeb");
    expect(adaptColorForDark("rgb(255, 200, 200)")).toBe("rgb(255, 200, 200)");
  });

  it("нераспознанный формат возвращает как есть", () => {
    expect(adaptColorForDark("rebeccapurple")).toBe("rebeccapurple");
  });

  it("rgb() с тёмными каналами осветляется", () => {
    const out = adaptColorForDark("rgb(0, 0, 139)");
    expect(out).toMatch(/^hsl\(/);
  });
});
