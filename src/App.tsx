// Каркас приложения: боковая навигация, активная страница, статус-бар, тосты.

import { Sidebar } from "@/components/Sidebar";
import { StatusBar } from "@/components/StatusBar";
import { Toaster } from "@/components/Toaster";
import { useDownloadsListener } from "@/hooks/useDownloads";
import { DownloadsPage } from "@/pages/DownloadsPage";
import { SearchPage } from "@/pages/SearchPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { TopicView } from "@/pages/TopicView";
import { useAppStore } from "@/store";

export default function App() {
  const view = useAppStore((s) => s.view);
  const topicId = useAppStore((s) => s.topicId);

  // Единый слушатель push-событий загрузок (скорости в статус-баре и т.д.).
  useDownloadsListener();

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-bg text-text">
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <main className="min-h-0 flex-1">
          {topicId !== null ? (
            <TopicView topicId={topicId} />
          ) : view === "search" ? (
            <SearchPage />
          ) : view === "downloads" ? (
            <DownloadsPage />
          ) : (
            <SettingsPage />
          )}
        </main>
        <StatusBar />
      </div>
      <Toaster />
    </div>
  );
}
