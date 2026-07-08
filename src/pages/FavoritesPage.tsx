// Страница избранного (данные подключаются в фазе 4).

import { Heart } from "lucide-react";

import { PageHeader } from "@/components/PageHeader";
import { EmptyState } from "@/components/ui";

export function FavoritesPage() {
  return (
    <div className="flex h-full flex-col">
      <PageHeader title="Избранное" />
      <div className="flex-1">
        <EmptyState
          icon={<Heart className="h-10 w-10" />}
          title="В избранном пока пусто"
          hint="Открывайте страницы раздач и добавляйте их в избранное — они появятся здесь, а приложение будет следить за их обновлениями."
        />
      </div>
    </div>
  );
}
