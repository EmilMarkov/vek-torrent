// Рендер первого поста раздачи в родной разметке rutracker. HTML приходит с
// бэкенда уже санированным (crates/rutracker/src/parse/sanitize.rs), поэтому
// вставка через dangerouslySetInnerHTML безопасна.

import { useMemo, useRef } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

import { openLightbox } from "@/components/Lightbox";
import { adaptColorForDark } from "@/lib/colors";
import { useAppStore } from "@/store";

// Приложение работает в защищённом контексте, и WKWebView режет http как mixed
// content — поднимаем схему до https (хостинги картинок её держат).
const upgradeHttp = (url: string) => url.replace(/^http:\/\//i, "https://");

// Готовит HTML поста до вставки: <var class="postImg"> → <img> (на трекере это
// делает их скрипт) и адаптирует авторские цвета к тёмной теме.
function preparePostHtml(html: string): string {
  const doc = new DOMParser().parseFromString(html, "text/html");

  doc.querySelectorAll("var.postImg").forEach((v) => {
    const src = v.getAttribute("title");
    if (!src) {
      v.remove();
      return;
    }
    const img = doc.createElement("img");
    img.src = src;
    img.className = v.className;
    img.alt = "";
    v.replaceWith(img);
  });

  doc.querySelectorAll("img[src]").forEach((img) => {
    const src = img.getAttribute("src") ?? "";
    if (src) img.setAttribute("src", upgradeHttp(src));
  });

  // Незакрытые шрифтовые/жирные спаны rutracker часто оборачивают целые блоки
  // поста (с <hr> и обтекаемым постером). WKWebView в таком строчном контексте
  // криво обтекает float — текст налезает на картинку. Верхнеуровневые спаны с
  // блочным содержимым переводим в BFC (flow-root) — обтекание становится верным.
  [...doc.body.children].forEach((el) => {
    if (
      el.tagName === "SPAN" &&
      el.querySelector("hr, img.img-left, img.img-right, img.postImgAligned")
    ) {
      (el as HTMLElement).style.display = "flow-root";
    }
  });

  doc.querySelectorAll<HTMLElement>("[style]").forEach((el) => {
    const color = el.style.color;
    if (!color) return;
    const adapted = adaptColorForDark(color);
    if (adapted === undefined) el.style.removeProperty("color");
    else if (adapted !== color) el.style.color = adapted;
  });

  return doc.body.innerHTML;
}

export function PostBody({ html }: { html: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const prepared = useMemo(() => preparePostHtml(html), [html]);

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

    if (img && root.contains(img) && !img.classList.contains("smile")) {
      e.preventDefault();
      // Полноразмерная картинка берётся из ссылки-обёртки — её схему тоже
      // поднимаем, иначе лайтбокс упрётся в mixed content.
      const full = link?.href && /\.(avif|gif|jpe?g|png|webp)(\?|$)/i.test(link.href);
      openLightbox(upgradeHttp(full ? link.href : img.currentSrc || img.src));
      return;
    }

    if (link && root.contains(link)) {
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
      className="post-body"
      onClick={onClick}
      dangerouslySetInnerHTML={{ __html: prepared }}
    />
  );
}

/** Идентификатор темы из ссылки вида …/viewtopic.php?t=123. */
function topicIdFromHref(href: string): number | null {
  if (!href.includes("viewtopic.php")) return null;
  const match = href.match(/[?&]t=(\d+)/);
  return match ? Number(match[1]) : null;
}
