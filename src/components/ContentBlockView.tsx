// Безопасный рендер блочной модели содержимого раздачи (без чужого HTML).

import { clsx } from "clsx";
import { ChevronRight, ImageOff } from "lucide-react";
import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

import { openLightbox } from "@/components/Lightbox";
import { useAppStore } from "@/store";
import type { ContentBlock, Inline } from "@/lib/types";

export function ContentBlocks({ blocks }: { blocks: ContentBlock[] }) {
  return (
    <div className="selectable flex flex-col gap-3 leading-relaxed">
      {blocks.map((block, i) => (
        <BlockView key={i} block={block} />
      ))}
    </div>
  );
}

function BlockView({ block }: { block: ContentBlock }) {
  switch (block.type) {
    case "paragraph":
      return (
        <p className="text-sm text-text/90">
          {block.inlines.map((inline, i) => (
            <InlineView key={i} inline={inline} />
          ))}
        </p>
      );
    case "image":
      return <BlockImage src={block.src} />;
    case "spoiler":
      return <Spoiler title={block.title} blocks={block.blocks} />;
    case "quote":
      return (
        <blockquote className="border-l-2 border-accent/50 bg-surface-2/60 py-2 pr-3 pl-4">
          {block.author && (
            <div className="mb-1 text-xs font-medium text-accent">{block.author}</div>
          )}
          <ContentBlocks blocks={block.blocks} />
        </blockquote>
      );
    case "code":
      return (
        <pre className="selectable overflow-x-auto rounded-lg border border-border bg-bg p-3 font-mono text-xs text-text/80">
          {block.text}
        </pre>
      );
    case "list":
      return <BlockList ordered={block.ordered} items={block.items} />;
    case "hr":
      return <hr className="border-border" />;
  }
}

function InlineView({ inline }: { inline: Inline }) {
  if (inline.type === "break") return <br />;

  if (inline.type === "image") return <InlineImage src={inline.src} />;

  if (inline.type === "link") {
    const openTopic = useAppStore.getState().openTopic;
    const onClick = () => {
      if (inline.topic_id != null) openTopic(inline.topic_id);
      else void openUrl(inline.href);
    };
    return (
      <button
        onClick={onClick}
        className="text-info underline decoration-info/40 underline-offset-2 hover:decoration-info"
      >
        {inline.text}
      </button>
    );
  }

  return (
    <span
      className={clsx(
        inline.bold && "font-semibold",
        inline.italic && "italic",
        inline.underline && "underline",
        inline.strike && "line-through",
      )}
      style={inline.color ? { color: sanitizeColor(inline.color) } : undefined}
    >
      {inline.text}
    </span>
  );
}

function BlockImage({ src }: { src: string }) {
  return <InlineImage src={src} />;
}

/** Строчное изображение: исходный размер, не разрывает строку, клик — лайтбокс. */
function InlineImage({ src }: { src: string }) {
  const [failed, setFailed] = useState(false);
  if (failed) {
    return (
      <span className="inline-flex items-center gap-1 align-middle text-xs text-faint">
        <ImageOff className="h-3.5 w-3.5" />
      </span>
    );
  }
  return (
    <img
      src={src}
      loading="lazy"
      onError={() => setFailed(true)}
      onClick={() => openLightbox(src)}
      // inline-block + align-middle: картинка течёт вместе с текстом; h-auto/
      // max-* не увеличивают маленькие изображения (флаги, спектрограммы).
      className="my-1 inline-block h-auto max-h-[70vh] max-w-full cursor-zoom-in rounded-md border border-border object-contain align-middle"
      alt=""
    />
  );
}

function Spoiler({ title, blocks }: { title: string; blocks: ContentBlock[] }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="overflow-hidden rounded-lg border border-border">
      <button
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center gap-1.5 bg-surface-2 px-3 py-2 text-left text-sm font-medium text-text hover:bg-surface-3"
      >
        <ChevronRight className={clsx("h-4 w-4 transition-transform", open && "rotate-90")} />
        {title}
      </button>
      {open && <div className="p-3">{<ContentBlocks blocks={blocks} />}</div>}
    </div>
  );
}

function BlockList({ ordered, items }: { ordered: boolean; items: ContentBlock[][] }) {
  const Tag = ordered ? "ol" : "ul";
  return (
    <Tag
      className={clsx("ml-5 flex flex-col gap-1 text-sm", ordered ? "list-decimal" : "list-disc")}
    >
      {items.map((blocks, i) => (
        <li key={i}>
          <ContentBlocks blocks={blocks} />
        </li>
      ))}
    </Tag>
  );
}

/** Пропускает только безопасные CSS-значения цвета. */
function sanitizeColor(color: string): string | undefined {
  return /^(#[0-9a-fA-F]{3,8}|[a-zA-Z]+|rgba?\([\d.,%\s]+\))$/.test(color) ? color : undefined;
}
