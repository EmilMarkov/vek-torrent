// Рендер первого поста раздачи в родной разметке rutracker.
//
// HTML приходит с бэкенда уже санированным (строгий whitelist тегов, классов
// и inline-стилей — crates/rutracker/src/parse/sanitize.rs), поэтому вставка
// через dangerouslySetInnerHTML безопасна. Стили классов rutracker
// (post-*, sp-*, q-*, c-*, postImg…) заданы в app.css под тёмную тему —
// авторская вёрстка (обтекание, таблицы, выравнивание) сохраняется 1:1.

import { useEffect, useRef } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

import { openLightbox } from "@/components/Lightbox";
import { adaptColorForDark } from "@/lib/colors";
import { useAppStore } from "@/store";

export function PostBody({ html }: { html: string }) {
  const ref = useRef<HTMLDivElement>(null);

  // Пост-обработка DOM после вставки HTML.
  useEffect(() => {
    const root = ref.current;
    if (!root) return;

    // 1. Изображения rutracker: <var class="postImg" title="URL"> → <img>.
    //    (На трекере это делает их скрипт — у нас скриптов из поста нет.)
    root.querySelectorAll("var.postImg").forEach((v) => {
      const src = v.getAttribute("title");
      if (!src) {
        v.remove();
        return;
      }
      const img = document.createElement("img");
      img.src = src;
      img.loading = "lazy";
      img.className = v.className;
      img.alt = "";
      v.replaceWith(img);
    });

    // 2. Авторские цвета подобраны под светлую тему трекера — адаптируем
    //    тёмные к тёмному фону (тон сохраняется, поднимается светлота).
    root.querySelectorAll<HTMLElement>("[style]").forEach((el) => {
      const color = el.style.color;
      if (!color) return;
      const adapted = adaptColorForDark(color);
      if (adapted === undefined) el.style.removeProperty("color");
      else if (adapted !== color) el.style.color = adapted;
    });
  }, [html]);

  // Делегированный клик: спойлеры, лайтбокс, перехват ссылок.
  const onClick = (e: React.MouseEvent) => {
    const root = ref.current;
    const target = e.target as HTMLElement;
    if (!root) return;

    const spHead = target.closest(".sp-head");
    if (spHead && root.contains(spHead)) {
      spHead.closest(".sp-wrap")?.classList.toggle("open");
      return;
    }

    const link = target.closest("a");
    const img = target.closest("img");

    // Клик по картинке: лайтбокс (для ссылки-обёртки на полноразмер — по ней).
    if (img && root.contains(img) && !img.classList.contains("smile")) {
      e.preventDefault();
      const full = link?.href && /\.(avif|gif|jpe?g|png|webp)(\?|$)/i.test(link.href);
      openLightbox(full ? link.href : img.currentSrc || img.src);
      return;
    }

    if (link && root.contains(link)) {
      // Навигация внутри webview недопустима — только приложение/браузер.
      e.preventDefault();
      const href = link.getAttribute("href") ?? "";
      const topicId = topicIdFromHref(href);
      if (topicId !== null) useAppStore.getState().openTopic(topicId);
      else if (href) void openUrl(href);
    }
  };

  return (
    <div
      ref={ref}
      className="post-body selectable"
      onClick={onClick}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}

/** Идентификатор темы из ссылки вида …/viewtopic.php?t=123. */
function topicIdFromHref(href: string): number | null {
  if (!href.includes("viewtopic.php")) return null;
  const match = href.match(/[?&]t=(\d+)/);
  return match ? Number(match[1]) : null;
}
