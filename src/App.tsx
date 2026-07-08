// Каркас приложения: боковая навигация, активная страница, статус-бар, тосты.

import { DownloadModal } from "@/components/DownloadModal";
import { Sidebar } from "@/components/Sidebar";
import { StatusBar } from "@/components/StatusBar";
import { Toaster } from "@/components/Toaster";
import { useDownloadsListener } from "@/hooks/useDownloads";
import { useFavoritesListener } from "@/hooks/useLibrary";
import { DownloadsPage } from "@/pages/DownloadsPage";
import { FavoritesPage } from "@/pages/FavoritesPage";
import { HistoryPage } from "@/pages/HistoryPage";
import { SearchPage } from "@/pages/SearchPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { TopicView } from "@/pages/TopicView";
import { useCurrentRoute } from "@/store";

export default function App() {
  const route = useCurrentRoute();

  // Единый слушатель push-событий загрузок (скорости в статус-баре и т.д.).
  useDownloadsListener();
  // Слушатель фоновых проверок обновлений избранного.
  useFavoritesListener();

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-bg text-text">
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <main className="min-h-0 flex-1">
          {route.kind === "topic" ? (
            <TopicView topicId={route.topicId} />
          ) : route.kind === "downloads" ? (
            <DownloadsPage />
          ) : route.kind === "favorites" ? (
            <FavoritesPage />
          ) : route.kind === "history" ? (
            <HistoryPage />
          ) : route.kind === "settings" ? (
            <SettingsPage />
          ) : (
            <SearchPage />
          )}
        </main>
        <StatusBar />
      </div>
      <Toaster />
      <DownloadModal />
    </div>
  );
}
