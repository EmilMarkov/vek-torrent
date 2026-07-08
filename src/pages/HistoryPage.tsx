// Страница истории скачиваний (данные подключаются в фазе 4).

import { History } from "lucide-react";

import { PageHeader } from "@/components/PageHeader";
import { EmptyState } from "@/components/ui";

export function HistoryPage() {
  return (
    <div className="flex h-full flex-col">
      <PageHeader title="История скачиваний" />
      <div className="flex-1">
        <EmptyState
          icon={<History className="h-10 w-10" />}
          title="История пуста"
          hint="Здесь будут отображаться раздачи, которые вы добавляли в загрузки."
        />
      </div>
    </div>
  );
}
