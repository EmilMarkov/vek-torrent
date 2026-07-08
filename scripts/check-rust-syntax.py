#!/usr/bin/env python3
"""Синтаксическая проверка всех .rs файлов через tree-sitter.

В среде разработки без rustc даёт быструю проверку синтаксиса
(полную компиляцию выполняет CI). Выход 1, если найдены ошибки.
"""

from __future__ import annotations

import sys
from pathlib import Path

import tree_sitter_rust
from tree_sitter import Language, Parser

LANGUAGE = Language(tree_sitter_rust.language())


def error_nodes(node, out):
    if node.type == "ERROR" or node.is_missing:
        out.append(node)
        return
    for child in node.children:
        error_nodes(child, out)


def main() -> int:
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(".")
    parser = Parser(LANGUAGE)
    failed = 0
    checked = 0

    for path in sorted(root.rglob("*.rs")):
        if "target" in path.parts or "node_modules" in path.parts:
            continue
        source = path.read_bytes()
        tree = parser.parse(source)
        errors: list = []
        error_nodes(tree.root_node, errors)
        checked += 1
        if errors:
            failed += 1
            for err in errors[:5]:
                line, col = err.start_point
                snippet = source[err.start_byte : err.start_byte + 60].decode(
                    "utf-8", "replace"
                )
                kind = "MISSING" if err.is_missing else "ERROR"
                print(f"{path}:{line + 1}:{col + 1}: {kind}: {snippet!r}")

    print(f"Проверено файлов: {checked}, с синтаксическими ошибками: {failed}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
