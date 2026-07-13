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

  // Пост-обработка вставленного HTML: превращаем <var class="postImg"> в <img>
  // (на трекере это делает их скрипт) и адаптируем авторские цвета к тёмной
  // теме. Наблюдаем за контейнером через MutationObserver: если React по
  // любой причине переустановит innerHTML (обновления соседних запросов после
  // «Отслеживать» и т.п.), <var>-картинки вернутся в необработанном виде —
  // обработчик повторно приведёт их к <img>, поэтому изображения не «теряются».
  useEffect(() => {
    const root = ref.current;
    if (!root) return;

    const process = () => {
      // Наши же замены (var→img) — это мутации childList; на время обработки
      // отключаем наблюдателя, чтобы не зациклиться.
      observer.disconnect();

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

      // Адаптируем только не помеченные элементы (идемпотентно при повторах).
      root.querySelectorAll<HTMLElement>("[style]:not([data-color-adapted])").forEach((el) => {
        el.setAttribute("data-color-adapted", "");
        const color = el.style.color;
        if (!color) return;
        const adapted = adaptColorForDark(color);
        if (adapted === undefined) el.style.removeProperty("color");
        else if (adapted !== color) el.style.color = adapted;
      });

      observer.observe(root, { childList: true, subtree: true });
    };

    const observer = new MutationObserver(process);
    process();
    return () => observer.disconnect();
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
