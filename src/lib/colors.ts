// Адаптация авторских цветов текста раздачи к тёмной теме.
//
// Авторы rutracker подбирают цвета под СВЕТЛЫЙ фон трекера: тёмно-синий,
// коричневый, чёрный и т.п. на нашем тёмном фоне нечитаемы. Сохраняем тон
// (hue) авторского цвета, но поднимаем светлоту до читаемого уровня.

/**
 * Порог максимального канала (0–255), ниже которого цвет — «почти чёрный»,
 * то есть на светлой теме играл роль цвета по умолчанию. Насыщенные тёмные
 * цвета (darkblue, maroon) под порог не попадают — их осветляем, а не сбрасываем.
 */
const NEAR_BLACK_CHANNEL = 40;
/** Воспринимаемая яркость, ниже которой цвет на тёмном фоне нечитаем. */
const MIN_READABLE_LUMA = 0.42;
/** Целевая светлота (HSL) после подъёма тёмного цвета. */
const LIFTED_L = 0.66;

/** Частые именованные CSS-цвета (палитра BB-кодов rutracker + базовые). */
const NAMED_COLORS: Record<string, [number, number, number]> = {
  black: [0, 0, 0],
  white: [255, 255, 255],
  gray: [128, 128, 128],
  grey: [128, 128, 128],
  silver: [192, 192, 192],
  red: [255, 0, 0],
  darkred: [139, 0, 0],
  crimson: [220, 20, 60],
  maroon: [128, 0, 0],
  orange: [255, 165, 0],
  darkorange: [255, 140, 0],
  gold: [255, 215, 0],
  yellow: [255, 255, 0],
  olive: [128, 128, 0],
  brown: [165, 42, 42],
  chocolate: [210, 105, 30],
  sienna: [160, 82, 45],
  green: [0, 128, 0],
  darkgreen: [0, 100, 0],
  seagreen: [46, 139, 87],
  lime: [0, 255, 0],
  teal: [0, 128, 128],
  aqua: [0, 255, 255],
  cyan: [0, 255, 255],
  skyblue: [135, 206, 235],
  steelblue: [70, 130, 180],
  royalblue: [65, 105, 225],
  blue: [0, 0, 255],
  mediumblue: [0, 0, 205],
  darkblue: [0, 0, 139],
  navy: [0, 0, 128],
  indigo: [75, 0, 130],
  violet: [238, 130, 238],
  darkviolet: [148, 0, 211],
  purple: [128, 0, 128],
  magenta: [255, 0, 255],
  fuchsia: [255, 0, 255],
  deeppink: [255, 20, 147],
  pink: [255, 192, 203],
};

/**
 * Приводит авторский цвет к читаемому на тёмном фоне.
 *
 * - почти чёрный → `undefined` (обычный цвет текста приложения);
 * - тёмный → тот же тон со светлотой, поднятой до читаемой;
 * - светлый/нераспознанный → без изменений.
 */
export function adaptColorForDark(color: string): string | undefined {
  const rgb = parseCssColor(color);
  if (!rgb) return color;

  if (Math.max(rgb[0], rgb[1], rgb[2]) < NEAR_BLACK_CHANNEL) return undefined;

  // Воспринимаемая яркость: жёлтый ярче синего при равной HSL-светлоте.
  const luma = (0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2]) / 255;
  if (luma >= MIN_READABLE_LUMA) return color;

  const [h, s, l] = rgbToHsl(rgb);
  return hslCss(h, s, Math.max(LIFTED_L, l));
}

/** Разбирает #rgb/#rrggbb, rgb()/rgba() и именованные цвета в [r, g, b]. */
function parseCssColor(color: string): [number, number, number] | null {
  const value = color.trim().toLowerCase();

  const named = NAMED_COLORS[value];
  if (named) return named;

  // Альфа-канал (#rgba/#rrggbbaa) игнорируется — важен только сам цвет.
  const hex = value.match(/^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/);
  if (hex) {
    const digits = hex[1];
    if (digits.length <= 4) {
      return [
        parseInt(digits[0] + digits[0], 16),
        parseInt(digits[1] + digits[1], 16),
        parseInt(digits[2] + digits[2], 16),
      ];
    }
    return [
      parseInt(digits.slice(0, 2), 16),
      parseInt(digits.slice(2, 4), 16),
      parseInt(digits.slice(4, 6), 16),
    ];
  }

  const rgb = value.match(/^rgba?\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})/);
  if (rgb) {
    const channels = [Number(rgb[1]), Number(rgb[2]), Number(rgb[3])];
    if (channels.every((c) => c <= 255)) return channels as [number, number, number];
  }

  return null;
}

/** RGB (0–255) → HSL (h 0–360, s/l 0–1). */
function rgbToHsl([r, g, b]: [number, number, number]): [number, number, number] {
  const rn = r / 255;
  const gn = g / 255;
  const bn = b / 255;
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const l = (max + min) / 2;

  if (max === min) return [0, 0, l];

  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h: number;
  if (max === rn) h = ((gn - bn) / d + (gn < bn ? 6 : 0)) * 60;
  else if (max === gn) h = ((bn - rn) / d + 2) * 60;
  else h = ((rn - gn) / d + 4) * 60;

  return [h, s, l];
}

function hslCss(h: number, s: number, l: number): string {
  return `hsl(${Math.round(h)}, ${Math.round(s * 100)}%, ${Math.round(l * 100)}%)`;
}
