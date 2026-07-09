import { describe, expect, it } from "vitest";

import { buildTree } from "./DownloadModal";

describe("buildTree", () => {
  it("группирует файлы по папкам и агрегирует размеры/индексы", () => {
    const root = buildTree([
      { index: 0, path: "Game/data/a.bin", size: 100 },
      { index: 1, path: "Game/data/b.bin", size: 200 },
      { index: 2, path: "Game/readme.txt", size: 10 },
      { index: 3, path: "root.txt", size: 5 },
    ]);

    // Верхний уровень: сперва папка Game, затем файл root.txt.
    expect(root.children.map((c) => c.kind)).toEqual(["folder", "file"]);
    expect(root.size).toBe(315);

    const game = root.children[0];
    if (game.kind !== "folder") throw new Error("ожидалась папка Game");
    expect(game.name).toBe("Game");
    expect(game.size).toBe(310);
    expect([...game.indices].sort((a, b) => a - b)).toEqual([0, 1, 2]);

    const data = game.children.find((c) => c.kind === "folder");
    if (!data || data.kind !== "folder") throw new Error("ожидалась вложенная папка data");
    expect(data.size).toBe(300);
    expect([...data.indices].sort((a, b) => a - b)).toEqual([0, 1]);
  });

  it("работает с плоским списком без папок", () => {
    const root = buildTree([
      { index: 0, path: "a.txt", size: 1 },
      { index: 1, path: "b.txt", size: 2 },
    ]);
    expect(root.children.every((c) => c.kind === "file")).toBe(true);
    expect(root.size).toBe(3);
  });
});
