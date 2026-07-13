// Каркас приложения: боковая навигация, активная страница, статус-бар, тосты.

import { DownloadModal } from "@/components/DownloadModal";
import { Lightbox } from "@/components/Lightbox";
import { Sidebar } from "@/components/Sidebar";
import { StatusBar } from "@/components/StatusBar";
import { Toaster } from "@/components/Toaster";
import { useDeepLink } from "@/hooks/useDeepLink";
import { useDownloadsListener } from "@/hooks/useDownloads";
import { useFavoritesListener } from "@/hooks/useLibrary";
import { CategoriesPage } from "@/pages/CategoriesPage";
import { DownloadsPage } from "@/pages/DownloadsPage";
import { ExternalTorrentsPage } from "@/pages/ExternalTorrentsPage";
import { FavoritesPage } from "@/pages/FavoritesPage";
import { FoldersPage } from "@/pages/FoldersPage";
import { HistoryPage } from "@/pages/HistoryPage";
import { SearchPage } from "@/pages/SearchPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { TopicView } from "@/pages/TopicView";
import { TrackedHistoryPage } from "@/pages/TrackedHistoryPage";
import { useCurrentRoute } from "@/store";

export default function App() {
  const route = useCurrentRoute();

  // Единый слушатель push-событий загрузок (скорости в статус-баре и т.д.).
  useDownloadsListener();
  // Слушатель фоновых проверок обновлений избранного.
  useFavoritesListener();
  // Обработка внутренних ссылок (переход на раздачу).
  useDeepLink();

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-bg text-text">
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <main className="min-h-0 flex-1">
          {route.kind === "topic" ? (
            <TopicView topicId={route.topicId} />
          ) : route.kind === "tracked-history" ? (
            <TrackedHistoryPage topicId={route.topicId} title={route.title} />
          ) : route.kind === "downloads" ? (
            <DownloadsPage />
          ) : route.kind === "favorites" ? (
            <FavoritesPage />
          ) : route.kind === "external" ? (
            <ExternalTorrentsPage />
          ) : route.kind === "folders" ? (
            <FoldersPage />
          ) : route.kind === "categories" ? (
            <CategoriesPage />
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
      <Lightbox />
    </div>
  );
}
