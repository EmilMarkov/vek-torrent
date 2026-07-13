// Страница «Категории»: метки для папок + наборы разделов rutracker,
// по которым работают чипы категорий в фильтрах поиска.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, Pencil, Plus, Tags, Trash2, X } from "lucide-react";
import { useState } from "react";
import { clsx } from "clsx";

import { ForumTreePicker, useForumGroups } from "@/components/ForumTreePicker";
import { PageHeader } from "@/components/PageHeader";
import { toast } from "@/components/Toaster";
import { Button, EmptyState, Input, Spinner } from "@/components/ui";
import { api, ApiError } from "@/lib/api";
import { effectiveCategoryForumIds } from "@/lib/filters";
import type { CategoryItem } from "@/lib/types";

/** Палитра цветов меток (в тон тёмной теме). */
export const CATEGORY_COLORS = [
  "#7c5cff",
  "#4da3ff",
  "#3ecf8e",
  "#f5b544",
  "#f4587a",
  "#ff8a5c",
  "#2dd4bf",
  "#e879f9",
  "#a3e635",
  "#94a3b8",
];

export function CategoriesPage() {
  const queryClient = useQueryClient();
  const { data, isLoading } = useQuery({
    queryKey: ["user-categories"],
    queryFn: api.userCategories,
  });

  const refresh = () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: ["user-categories"] }),
      // Категории развёрнуты внутри папок — их тоже обновляем.
      queryClient.invalidateQueries({ queryKey: ["folders"] }),
    ]);

  const categories = data ?? [];

  return (
    <div className="flex h-full flex-col">
      <PageHeader title="Категории" />
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto flex max-w-2xl flex-col gap-4 px-6 py-5">
          <CreateCategory onDone={refresh} />

          {isLoading ? (
            <div className="flex justify-center py-16">
              <Spinner className="h-6 w-6" />
            </div>
          ) : categories.length === 0 ? (
            <EmptyState
              icon={<Tags className="h-10 w-10" />}
              title="Категорий нет"
              hint="Создайте категорию, чтобы помечать ею папки с раздачами."
            />
          ) : (
            <div className="flex flex-col divide-y divide-border/60 rounded-xl border border-border bg-surface">
              {categories.map((category) => (
                <CategoryRow key={category.id} category={category} onDone={refresh} />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

/** Выбор цвета метки из палитры. */
function ColorPicker({ value, onChange }: { value: string; onChange: (color: string) => void }) {
  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {CATEGORY_COLORS.map((color) => (
        <button
          key={color}
          type="button"
          onClick={() => onChange(color)}
          title={color}
          className={clsx(
            "h-6 w-6 rounded-full transition-transform hover:scale-110",
            value === color && "ring-2 ring-text ring-offset-2 ring-offset-surface",
          )}
          style={{ backgroundColor: color }}
        />
      ))}
    </div>
  );
}

function CreateCategory({ onDone }: { onDone: () => void }) {
  const [name, setName] = useState("");
  const [color, setColor] = useState(CATEGORY_COLORS[0]);
  const [forumIds, setForumIds] = useState<number[]>([]);
  const [showForums, setShowForums] = useState(false);
  const [saving, setSaving] = useState(false);

  const create = async () => {
    // saving-guard: повторный Enter до ре-рендера не должен дублировать запись.
    if (!name.trim() || saving) return;
    setSaving(true);
    try {
      await api.addUserCategory(name.trim(), color, forumIds);
      setName("");
      setForumIds([]);
      setShowForums(false);
      toast.success("Категория создана");
      onDone();
    } catch (error) {
      toast.error(error instanceof ApiError ? error.message : String(error));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="flex flex-col gap-3 rounded-xl border border-border bg-surface p-4">
      <span className="text-sm font-semibold text-text">Новая категория</span>
      <div className="flex gap-2">
        <Input
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && void create()}
          placeholder="Название категории"
        />
        <Button variant="primary" loading={saving} onClick={() => void create()}>
          <Plus className="h-4 w-4" />
          Создать
        </Button>
      </div>
      <ColorPicker value={color} onChange={setColor} />
      <button
        onClick={() => setShowForums((v) => !v)}
        className="self-start text-xs text-info hover:underline"
      >
        {showForums ? "Скрыть разделы" : `Разделы rutracker (${forumIds.length})`}
      </button>
      {showForums && (
        <ForumTreePicker
          selected={forumIds}
          onChange={setForumIds}
          label="Разделы категории (для фильтров поиска)"
        />
      )}
    </div>
  );
}

function CategoryRow({ category, onDone }: { category: CategoryItem; onDone: () => void }) {
  const forumGroups = useForumGroups();
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(category.name);
  const [color, setColor] = useState(category.color);
  const [forumIds, setForumIds] = useState<number[]>(category.forumIds);

  // Разделы, которые категория фактически охватывает: явно заданные или, для
  // стандартной категории без настройки, разрешённые эвристикой по имени.
  const effectiveForumIds = effectiveCategoryForumIds(forumGroups.data ?? [], category);

  // Синхронизация при входе в редактирование: состояние инициализируется на
  // маунте и без этого не подхватило бы актуальные разделы из props. Для
  // стандартной категории (пустой forumIds) показываем эвристический набор
  // отмеченным — сохранение зафиксирует его как явный.
  const startEdit = () => {
    setName(category.name);
    setColor(category.color);
    setForumIds(effectiveForumIds);
    setEditing(true);
  };

  const save = async () => {
    if (!name.trim()) return;
    try {
      await api.updateUserCategory(category.id, name.trim(), color, forumIds);
      setEditing(false);
      onDone();
    } catch (error) {
      toast.error(error instanceof ApiError ? error.message : String(error));
    }
  };

  const remove = async () => {
    await api.removeUserCategory(category.id);
    toast.info(`Категория «${category.name}» удалена`);
    onDone();
  };

  if (editing) {
    return (
      <div className="flex flex-col gap-3 px-4 py-3">
        <div className="flex items-center gap-2">
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && void save()}
            autoFocus
          />
          <Button variant="primary" onClick={() => void save()} title="Сохранить">
            <Check className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            onClick={() => {
              setEditing(false);
              setName(category.name);
              setColor(category.color);
              setForumIds(category.forumIds);
            }}
            title="Отмена"
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
        <ColorPicker value={color} onChange={setColor} />
        <ForumTreePicker
          selected={forumIds}
          onChange={setForumIds}
          label="Разделы категории (для фильтров поиска)"
        />
      </div>
    );
  }

  return (
    <div className="group flex items-center gap-3 px-4 py-3">
      <span
        className="h-3.5 w-3.5 shrink-0 rounded-full"
        style={{ backgroundColor: category.color }}
      />
      <span className="min-w-0 flex-1 truncate text-sm text-text">{category.name}</span>
      <span className="shrink-0 text-xs text-faint">
        {category.forumIds.length > 0
          ? `разделов: ${category.forumIds.length}`
          : "разделы по ключевым словам"}
      </span>
      <Button
        variant="ghost"
        onClick={startEdit}
        title="Изменить (название, цвет, разделы)"
        className="opacity-0 group-hover:opacity-100"
      >
        <Pencil className="h-4 w-4" />
      </Button>
      <Button
        variant="ghost"
        onClick={() => void remove()}
        title="Удалить категорию"
        className="opacity-0 group-hover:opacity-100"
      >
        <Trash2 className="h-4 w-4 text-danger" />
      </Button>
    </div>
  );
}
