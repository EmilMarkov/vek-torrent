// Утилиты форматирования для UI.

const SIZE_UNITS = ["Б", "КБ", "МБ", "ГБ", "ТБ", "ПБ"];

/** Форматирует размер в байтах в человекочитаемый вид. */
export function formatSize(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 Б";
  const exp = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), SIZE_UNITS.length - 1);
  const value = bytes / Math.pow(1024, exp);
  const digits = value >= 100 || exp === 0 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(digits)} ${SIZE_UNITS[exp]}`;
}

/** Форматирует скорость (байт/с). */
export function formatSpeed(bytesPerSec: number): string {
  if (bytesPerSec <= 0) return "—";
  return `${formatSize(bytesPerSec)}/с`;
}

/** Форматирует оставшееся время (секунды). */
export function formatEta(seconds: number | null): string {
  if (seconds === null || seconds <= 0) return "∞";
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  if (d > 0) return `${d}д ${h}ч`;
  if (h > 0) return `${h}ч ${m}м`;
  if (m > 0) return `${m}м ${s}с`;
  return `${s}с`;
}

/** Unix-время (секунды) → локальная дата. */
export function formatDate(unix: number): string {
  if (!unix || unix <= 0) return "—";
  return new Date(unix * 1000).toLocaleDateString("ru-RU", {
    day: "2-digit",
    month: "short",
    year: "numeric",
  });
}

/** Число с разделителями разрядов. */
export function formatNumber(value: number): string {
  return value.toLocaleString("ru-RU");
}
