// Модалка выбора файлов перед скачиванием — древовидная структура с папками.
//
// Глобальный стор: любой компонент вызывает openDownloadModal(topicId, title),
// а сама модалка монтируется один раз в App.

import { useQuery } from "@tanstack/react-query";
import {
  CheckSquare,
  ChevronDown,
  ChevronRight,
  File as FileIcon,
  Folder as FolderIcon,
  FolderOpen,
  MinusSquare,
  Square,
  TriangleAlert,
} from "lucide-react";
import { useMemo, useState } from "react";
import { create } from "zustand";

import { toast } from "@/components/Toaster";
import { Button, Spinner, Toggle } from "@/components/ui";
import { api, ApiError } from "@/lib/api";
import { formatSize } from "@/lib/format";
import type { TorrentFile } from "@/lib/types";
import { useAppStore } from "@/store";

interface ModalStore {
  topicId: number | null;
  title: string;
  open: (topicId: number, title: string) => void;
  close: () => void;
}

const useModalStore = create<ModalStore>((set) => ({
  topicId: null,
  title: "",
  open: (topicId, title) => set({ topicId, title }),
  close: () => set({ topicId: null }),
}));

/** Открыть модалку выбора файлов для раздачи. */
export function openDownloadModal(topicId: number, title: string) {
  useModalStore.getState().open(topicId, title);
}

export function DownloadModal() {
  const topicId = useModalStore((s) => s.topicId);
  const title = useModalStore((s) => s.title);
  const close = useModalStore((s) => s.close);

  if (topicId === null) return null;
  return <ModalBody topicId={topicId} title={title} onClose={close} />;
}

// ── Дерево файлов ────────────────────────────────────────────────────────────

interface FileNode {
  kind: "file";
  name: string;
  index: number;
  size: number;
}
interface FolderNode {
  kind: "folder";
  name: string;
  path: string;
  size: number;
  indices: number[];
  children: TreeNode[];
}
type TreeNode = FileNode | FolderNode;

/** Строит дерево папок из плоского списка файлов (путь через «/»). */
export function buildTree(files: TorrentFile[]): FolderNode {
  const root: FolderNode = {
    kind: "folder",
    name: "",
    path: "",
    size: 0,
    indices: [],
    children: [],
  };
  const folders = new Map<string, FolderNode>([["", root]]);

  for (const file of files) {
    const parts = file.path.split("/").filter(Boolean);
    let parent = root;
    let acc = "";
    for (let i = 0; i < parts.length - 1; i++) {
      acc = acc ? `${acc}/${parts[i]}` : parts[i];
      let folder = folders.get(acc);
      if (!folder) {
        folder = { kind: "folder", name: parts[i], path: acc, size: 0, indices: [], children: [] };
        folders.set(acc, folder);
        parent.children.push(folder);
      }
      parent = folder;
    }
    parent.children.push({
      kind: "file",
      name: parts[parts.length - 1] ?? file.path,
      index: file.index,
      size: file.size,
    });
  }

  const finalize = (node: FolderNode) => {
    for (const child of node.children) {
      if (child.kind === "folder") {
        finalize(child);
        node.indices.push(...child.indices);
      } else {
        node.indices.push(child.index);
      }
      node.size += child.size;
    }
    // Папки выше файлов, внутри — по алфавиту.
    node.children.sort((a, b) =>
      a.kind === b.kind ? a.name.localeCompare(b.name) : a.kind === "folder" ? -1 : 1,
    );
  };
  finalize(root);
  return root;
}

interface Row {
  node: TreeNode;
  depth: number;
}

/** Разворачивает дерево в плоский список строк с учётом раскрытых папок. */
function flatten(nodes: TreeNode[], depth: number, expanded: Set<string>, out: Row[]) {
  for (const node of nodes) {
    out.push({ node, depth });
    if (node.kind === "folder" && expanded.has(node.path)) {
      flatten(node.children, depth + 1, expanded, out);
    }
  }
}

function ModalBody({
  topicId,
  title,
  onClose,
}: {
  topicId: number;
  title: string;
  onClose: () => void;
}) {
  const setView = useAppStore((s) => s.setView);
  // null — «пользователь ничего не менял» → выбраны все файлы по умолчанию.
  const [custom, setCustom] = useState<Set<number> | null>(null);
  // По умолчанию все папки свёрнуты (раскрытые — в этом множестве).
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [stopped, setStopped] = useState(false);
  const [adding, setAdding] = useState(false);

  const { data, isLoading, error } = useQuery({
    queryKey: ["topic-files", topicId],
    queryFn: () => api.topicFiles(topicId),
    retry: false,
  });

  const allIndices = useMemo(() => new Set(data?.files.map((f) => f.index) ?? []), [data]);
  const tree = useMemo(() => (data ? buildTree(data.files) : null), [data]);
  const rows = useMemo(() => {
    if (!tree) return [];
    const out: Row[] = [];
    flatten(tree.children, 0, expanded, out);
    return out;
  }, [tree, expanded]);

  // Эффективный выбор: пользовательский либо «все» (без эффектов синхронизации).
  const selected = custom ?? allIndices;

  const selectedSize = useMemo(
    () =>
      data ? data.files.filter((f) => selected.has(f.index)).reduce((s, f) => s + f.size, 0) : 0,
    [data, selected],
  );

  const setMany = (indices: number[], select: boolean) =>
    setCustom((prev) => {
      const next = new Set(prev ?? allIndices);
      for (const index of indices) {
        if (select) next.add(index);
        else next.delete(index);
      }
      return next;
    });

  const toggleExpand = (path: string) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });

  const allSelected = data ? data.files.length > 0 && selected.size === data.files.length : false;
  const toggleAll = () => setMany([...allIndices], !allSelected);

  const submit = async (onlyFiles: number[] | null) => {
    setAdding(true);
    try {
      await api.addFromTopic(topicId, { onlyFiles, stopped });
      toast.success("Добавлено в загрузки");
      onClose();
      setView("downloads");
    } catch (e) {
      toast.error(e instanceof ApiError ? describe(e) : String(e));
    } finally {
      setAdding(false);
    }
  };

  const confirm = () => {
    if (!data || selected.size === 0) return;
    void submit(allSelected ? null : [...selected]);
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      onClick={onClose}
    >
      <div
        className="flex max-h-[80vh] w-full max-w-2xl flex-col rounded-xl border border-border bg-surface"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="border-b border-border px-5 py-3">
          <h3 className="truncate text-base font-semibold text-text">Выбор файлов</h3>
          <p className="mt-0.5 truncate text-xs text-faint">{data?.name || title}</p>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto">
          {isLoading ? (
            <div className="flex h-40 items-center justify-center">
              <Spinner className="h-6 w-6" />
            </div>
          ) : error || !data ? (
            <div className="flex h-48 flex-col items-center justify-center gap-3 px-6 text-center">
              <TriangleAlert className="h-8 w-8 text-warn" />
              <div>
                <p className="text-sm text-muted">Не удалось получить список файлов</p>
                <p className="mt-1 text-xs text-faint">
                  {error instanceof ApiError ? describe(error) : "Проверьте вход на rutracker."}
                </p>
              </div>
              {/* Запасной путь: добавить раздачу целиком, без выбора файлов. */}
              <Button variant="secondary" loading={adding} onClick={() => void submit(null)}>
                Скачать все файлы
              </Button>
            </div>
          ) : (
            <div className="p-2">
              <button
                onClick={toggleAll}
                className="mb-1 flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs font-medium text-muted hover:bg-surface-2"
              >
                {allSelected ? (
                  <CheckSquare className="h-4 w-4 text-accent" />
                ) : (
                  <Square className="h-4 w-4" />
                )}
                {allSelected ? "Снять все" : "Выбрать все"} ({data.files.length})
              </button>
              {rows.map(({ node, depth }) => (
                <TreeRow
                  key={node.kind === "folder" ? `d:${node.path}` : `f:${node.index}`}
                  node={node}
                  depth={depth}
                  selected={selected}
                  expanded={expanded}
                  onToggle={setMany}
                  onExpand={toggleExpand}
                />
              ))}
            </div>
          )}
        </div>

        <div className="flex items-center gap-3 border-t border-border px-5 py-3">
          <Toggle checked={stopped} onChange={setStopped} label="На паузе" />
          <div className="ml-auto flex items-center gap-3">
            {data && (
              <span className="text-xs text-faint">
                {selected.size} из {data.files.length} · {formatSize(selectedSize)}
              </span>
            )}
            <Button variant="ghost" onClick={onClose}>
              Отмена
            </Button>
            <Button
              variant="primary"
              loading={adding}
              disabled={!data || selected.size === 0}
              onClick={confirm}
            >
              Скачать
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}

function TreeRow({
  node,
  depth,
  selected,
  expanded,
  onToggle,
  onExpand,
}: {
  node: TreeNode;
  depth: number;
  selected: Set<number>;
  expanded: Set<string>;
  onToggle: (indices: number[], select: boolean) => void;
  onExpand: (path: string) => void;
}) {
  // Чекбоксы стоят единой колонкой слева (без отступа); вложенность сдвигает
  // только метку — иконку с названием.
  const indent = { paddingLeft: depth * 16 };

  if (node.kind === "file") {
    const checked = selected.has(node.index);
    return (
      <div className="flex items-center gap-1.5 rounded-md px-2 py-1 hover:bg-surface-2">
        <button
          onClick={() => onToggle([node.index], !checked)}
          className="shrink-0"
          title={checked ? "Не скачивать" : "Скачивать"}
        >
          {checked ? (
            <CheckSquare className="h-4 w-4 text-accent" />
          ) : (
            <Square className="h-4 w-4 text-faint" />
          )}
        </button>
        <div className="flex min-w-0 flex-1 items-center gap-1.5" style={indent}>
          <span className="w-3.5 shrink-0" />
          <FileIcon className="h-4 w-4 shrink-0 text-faint" />
          <span className="min-w-0 flex-1 truncate text-sm text-text/90" title={node.name}>
            {node.name}
          </span>
        </div>
        <span className="shrink-0 text-xs text-faint">{formatSize(node.size)}</span>
      </div>
    );
  }

  const chosen = node.indices.reduce((n, i) => n + (selected.has(i) ? 1 : 0), 0);
  const state = chosen === 0 ? "none" : chosen === node.indices.length ? "all" : "some";
  const open = expanded.has(node.path);

  return (
    <div className="flex items-center gap-1.5 rounded-md px-2 py-1 hover:bg-surface-2">
      <button
        onClick={() => onToggle(node.indices, state !== "all")}
        className="shrink-0"
        title={state === "all" ? "Снять папку" : "Выбрать папку"}
      >
        {state === "all" ? (
          <CheckSquare className="h-4 w-4 text-accent" />
        ) : state === "some" ? (
          <MinusSquare className="h-4 w-4 text-accent" />
        ) : (
          <Square className="h-4 w-4 text-faint" />
        )}
      </button>
      <button
        onClick={() => onExpand(node.path)}
        className="flex min-w-0 flex-1 items-center gap-1.5 text-left"
        style={indent}
      >
        {open ? (
          <ChevronDown className="h-3.5 w-3.5 shrink-0 text-faint" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5 shrink-0 text-faint" />
        )}
        {open ? (
          <FolderOpen className="h-4 w-4 shrink-0 text-accent/80" />
        ) : (
          <FolderIcon className="h-4 w-4 shrink-0 text-accent/80" />
        )}
        <span className="min-w-0 flex-1 truncate text-sm font-medium text-text" title={node.name}>
          {node.name}
        </span>
      </button>
      <span className="shrink-0 text-xs text-faint">{formatSize(node.size)}</span>
    </div>
  );
}

function describe(error: ApiError): string {
  switch (error.code) {
    case "not_authenticated":
      return "Требуется вход на rutracker (см. Настройки).";
    case "engine_unavailable":
      return "Торрент-движок не запущен — включите его в Настройках.";
    default:
      // Показываем настоящую причину (например, таймаут получения метаданных).
      return error.message;
  }
}
