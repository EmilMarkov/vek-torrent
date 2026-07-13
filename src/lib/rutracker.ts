// Ссылки на rutracker.
//
// Для «поделиться» используется канонический rutracker.org (получатель сам
// решает, как обходить блокировки), для открытия в браузере — зеркало из
// настроек: у пользователя канонический домен может быть заблокирован.

const CANONICAL_BASE = "https://rutracker.org";

/** URL страницы раздачи. `mirror` — базовый адрес зеркала из настроек. */
export function rutrackerTopicUrl(topicId: number, mirror?: string): string {
  const base =
    (mirror ?? "")
      .trim()
      .replace(/\/+$/, "")
      .replace(/\/forum$/, "") || CANONICAL_BASE;
  return `${base}/forum/viewtopic.php?t=${topicId}`;
}
