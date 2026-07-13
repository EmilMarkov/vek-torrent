// Страница «Папки» — в структуре страницы поиска: строка поиска сверху,
// список во всю ширину, фильтры в правом сайдбаре.
//
// Категория папки помечается визуально: цветная иконка папки и бейдж с
// названием категории в её цвете.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  Check,
  Folder as FolderIcon,
  FolderPlus,
  Pencil,
  Search as SearchIcon,
  SlidersHorizontal,
  Trash2,
  X,
} from "lucide-react";
import { useMemo, useState } from "react";
import { clsx } from "clsx";

import { toast } from "@/components/Toaster";
import { Button, EmptyState, Input, Select, Spinner } from "@/components/ui";
import { api, ApiError } from "@/lib/api";
import { formatDate } from "@/lib/format";
import type { CategoryItem, FolderItem } from "@/lib/types";
import { useAppStore } from "@/store";

export function FoldersPage() {
  const queryClient = useQueryClient();
  const { data: folders, isLoading } = useQuery({ queryKey: ["folders"], queryFn: api.folders });
  const { data: categories } = useQuery({
    queryKey: ["user-categories"],
    queryFn: api.userCategories,
  });

  const [openedId, setOpenedId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [showFilters, setShowFilters] = useState(true);

  // Фильтры списка папок.
  const [query, setQuery] = useState("");
  const [categoryFilter, setCategoryFilter] = useState("");

  const refresh = () => queryClient.invalidateQueries({ queryKey: ["folders"] });

  const all = useMemo(() => folders ?? [], [folders]);
  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return all.filter((f) => {
      if (q && !f.name.toLowerCase().includes(q)) return false;
      if (categoryFilter === "none" && f.category !== null) return false;
      if (categoryFilter && categoryFilter !== "none" && f.category?.id !== categoryFilter)
        return false;
      return true;
    });
  }, [all, query, categoryFilter]);

  const opened = all.find((f) => f.id === openedId) ?? null;
  if (opened) {
    return <FolderView folder={opened} onBack={() => setOpenedId(null)} onChanged={refresh} />;
  }

  const filtersActive = categoryFilter !== "";

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center gap-2 border-b border-border px-5 py-4">
        <div className="relative flex-1">
          <SearchIcon className="pointer-events-none absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 text-faint" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Поиск по папкам…"
            className="pl-9"
          />
        </div>
        <Button variant="primary" onClick={() => setCreating((v) => !v)}>
          <FolderPlus className="h-4 w-4" />
          Новая папка
        </Button>
        <Button
          variant={showFilters || filtersActive ? "primary" : "secondary"}
          onClick={() => setShowFilters((v) => !v)}
        >
          <SlidersHorizontal className="h-4 w-4" />
          Фильтры
        </Button>
      </header>

      <div className="flex min-h-0 flex-1">
        <div className="min-w-0 flex-1 overflow-y-auto">
          {creating && (
            <div className="border-b border-border px-5 py-4">
              <CreateFolder
                categories={categories ?? []}
                onDone={() => {
                  setCreating(false);
                  refresh();
                }}
              />
            </div>
          )}

          {isLoading ? (
            <div className="flex justify-center py-16">
              <Spinner className="h-6 w-6" />
            </div>
          ) : all.length === 0 ? (
            <EmptyState
              icon={<FolderIcon className="h-10 w-10" />}
              title="Папок пока нет"
              hint="Создайте папку и добавляйте в неё раздачи кнопкой «В папку» на странице раздачи."
            />
          ) : visible.length === 0 ? (
            <EmptyState
              icon={<SearchIcon className="h-10 w-10" />}
              title="Ничего не найдено"
              hint="Попробуйте изменить фильтры."
            />
          ) : (
            <div className="flex flex-col divide-y divide-border/60 px-3">
              {visible.map((folder) => (
                <FolderRow
                  key={folder.id}
                  folder={folder}
                  categories={categories ?? []}
                  onOpen={() => setOpenedId(folder.id)}
                  onChanged={refresh}
                />
              ))}
            </div>
          )}
        </div>

        {showFilters && (
          <aside className="flex w-72 shrink-0 flex-col overflow-y-auto border-l border-border bg-surface/40">
            <div className="flex items-center justify-between border-b border-border px-4 py-3">
              <span className="text-sm font-semibold text-text">Фильтры</span>
              {filtersActive && (
                <button
                  onClick={() => setCategoryFilter("")}
                  className="inline-flex items-center gap-1 text-xs text-faint hover:text-danger"
                >
                  <X className="h-3.5 w-3.5" />
                  Сбросить
                </button>
              )}
            </div>
            <div className="flex flex-col gap-4 p-4">
              <div className="flex flex-col gap-1.5">
                <span className="text-[11px] font-medium text-faint">Категория</span>
                <div className="flex flex-wrap gap-1.5">
                  <FilterChip
                    label="Без категории"
                    active={categoryFilter === "none"}
                    onClick={() => setCategoryFilter(categoryFilter === "none" ? "" : "none")}
                  />
                  {(categories ?? []).map((category) => (
                    <FilterChip
                      key={category.id}
                      label={category.name}
                      color={category.color}
                      active={categoryFilter === category.id}
                      onClick={() =>
                        setCategoryFilter(categoryFilter === category.id ? "" : category.id)
                      }
                    />
                  ))}
                </div>
              </div>
              <span className="text-xs text-faint">
                Показано {visible.length} из {all.length}
              </span>
            </div>
          </aside>
        )}
      </div>
    </div>
  );
}

/** Чип фильтра по категории (одиночный выбор). */
function FilterChip({
  label,
  color,
  active,
  onClick,
}: {
  label: string;
  color?: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={
        active
          ? "rounded-full px-3 py-1 text-xs font-medium"
          : "rounded-full border border-border px-3 py-1 text-xs text-muted hover:border-border-strong hover:text-text"
      }
      style={
        active
          ? color
            ? { backgroundColor: `${color}26`, color }
            : { backgroundColor: "var(--color-accent-soft)", color: "var(--color-accent)" }
          : undefined
      }
    >
      {label}
    </button>
  );
}

/** Бейдж категории в её цвете. */
export function CategoryBadge({ category }: { category: CategoryItem }) {
  return (
    <span
      className="inline-flex shrink-0 items-center gap-1.5 rounded-md px-1.5 py-0.5 text-[11px] font-medium"
      style={{ backgroundColor: `${category.color}26`, color: category.color }}
    >
      <span className="h-2 w-2 rounded-full" style={{ backgroundColor: category.color }} />
      {category.name}
    </span>
  );
}

/** Варианты для селекта категории («без категории» + пользовательские). */
function categoryOptions(categories: CategoryItem[]) {
  return [
    { value: "", label: "Без категории" },
    ...categories.map((c) => ({ value: c.id, label: c.name })),
  ];
}

function CreateFolder({ categories, onDone }: { categories: CategoryItem[]; onDone: () => void }) {
  const [name, setName] = useState("");
  const [categoryId, setCategoryId] = useState("");
  const [saving, setSaving] = useState(false);

  const create = async () => {
    // saving-guard: повторный Enter до ре-рендера не должен дублировать папку.
    if (!name.trim() || saving) return;
    setSaving(true);
    try {
      await api.addFolder(name.trim(), categoryId || null);
      setName("");
      setCategoryId("");
      toast.success("Папка создана");
      onDone();
    } catch (error) {
      toast.error(error instanceof ApiError ? error.message : String(error));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="flex gap-2">
      <Input
        value={name}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && void create()}
        placeholder="Название папки"
        autoFocus
      />
      <Select
        className="w-44 shrink-0"
        value={categoryId}
        onChange={setCategoryId}
        options={categoryOptions(categories)}
      />
      <Button variant="primary" loading={saving} onClick={() => void create()}>
        Создать
      </Button>
    </div>
  );
}

function FolderRow({
  folder,
  categories,
  onOpen,
  onChanged,
}: {
  folder: FolderItem;
  categories: CategoryItem[];
  onOpen: () => void;
  onChanged: () => void;
}) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(folder.name);
  const [categoryId, setCategoryId] = useState(folder.category?.id ?? "");

  // Синхронизация при входе в редактирование (состояние с маунта могло
  // устареть после обновления props).
  const startEdit = () => {
    setName(folder.name);
    setCategoryId(folder.category?.id ?? "");
    setEditing(true);
  };

  const save = async () => {
    if (!name.trim()) return;
    try {
      await api.updateFolder(folder.id, name.trim(), categoryId || null);
      setEditing(false);
      onChanged();
    } catch (error) {
      toast.error(error instanceof ApiError ? error.message : String(error));
    }
  };

  const remove = async () => {
    await api.removeFolder(folder.id);
    toast.info(`Папка «${folder.name}» удалена`);
    onChanged();
  };

  if (editing) {
    return (
      <div className="flex items-center gap-2 py-2.5">
        <Input
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && void save()}
          autoFocus
        />
        <Select
          className="w-44 shrink-0"
          value={categoryId}
          onChange={setCategoryId}
          options={categoryOptions(categories)}
        />
        <Button variant="primary" onClick={() => void save()} title="Сохранить">
          <Check className="h-4 w-4" />
        </Button>
        <Button variant="ghost" onClick={() => setEditing(false)} title="Отмена">
          <X className="h-4 w-4" />
        </Button>
      </div>
    );
  }

  return (
    <div className="group flex items-center gap-3 py-2.5">
      {/* Цвет иконки папки — визуальная пометка категории. */}
      <FolderIcon
        className="h-5 w-5 shrink-0"
        style={{ color: folder.category?.color ?? "var(--color-faint)" }}
      />
      <button onClick={onOpen} className="min-w-0 flex-1 text-left">
        <div className="flex items-center gap-2">
          <span className="truncate text-sm font-medium text-text group-hover:text-accent">
            {folder.name}
          </span>
          {folder.category && <CategoryBadge category={folder.category} />}
        </div>
      </button>
      <span className={clsx("w-24 shrink-0 text-right text-xs text-muted")}>
        {folder.topics.length === 0 ? "пусто" : `раздач: ${folder.topics.length}`}
      </span>
      <span className="w-24 shrink-0 text-right text-xs text-faint max-lg:hidden">
        {formatDate(folder.createdAt)}
      </span>
      <div className="flex w-20 shrink-0 justify-end gap-1">
        <Button
          variant="ghost"
          onClick={startEdit}
          title="Переименовать / категория"
          className="opacity-0 group-hover:opacity-100"
        >
          <Pencil className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          onClick={() => void remove()}
          title="Удалить папку"
          className="opacity-0 group-hover:opacity-100"
        >
          <Trash2 className="h-4 w-4 text-danger" />
        </Button>
      </div>
    </div>
  );
}

/** Содержимое папки: список раздач. */
function FolderView({
  folder,
  onBack,
  onChanged,
}: {
  folder: FolderItem;
  onBack: () => void;
  onChanged: () => void;
}) {
  const openTopic = useAppStore((s) => s.openTopic);

  const removeTopic = async (topicId: number) => {
    await api.removeTopicFromFolder(folder.id, topicId);
    onChanged();
  };

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center gap-3 border-b border-border px-5 py-3">
        <Button variant="ghost" onClick={onBack} className="px-2">
          <ArrowLeft className="h-4 w-4" />
          Папки
        </Button>
        <FolderIcon
          className="h-5 w-5 shrink-0"
          style={{ color: folder.category?.color ?? "var(--color-faint)" }}
        />
        <h1 className="truncate text-base font-semibold text-text">{folder.name}</h1>
        {folder.category && <CategoryBadge category={folder.category} />}
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto">
        {folder.topics.length === 0 ? (
          <EmptyState
            icon={<FolderIcon className="h-10 w-10" />}
            title="Папка пуста"
            hint="Добавляйте раздачи кнопкой «В папку» на странице раздачи."
          />
        ) : (
          <div className="flex flex-col divide-y divide-border/60 px-3">
            {folder.topics.map((topic) => (
              <div key={topic.topicId} className="group flex items-center gap-3 py-2.5">
                <button
                  onClick={() => openTopic(topic.topicId)}
                  className="min-w-0 flex-1 text-left"
                >
                  <span className="truncate text-sm font-medium text-text group-hover:text-accent">
                    {topic.title}
                  </span>
                  <div className="mt-0.5 text-xs text-faint">
                    добавлена {formatDate(topic.addedAt)}
                  </div>
                </button>
                <Button
                  variant="ghost"
                  onClick={() => void removeTopic(topic.topicId)}
                  title="Убрать из папки"
                  className="shrink-0 opacity-0 group-hover:opacity-100"
                >
                  <Trash2 className="h-4 w-4 text-danger" />
                </Button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
