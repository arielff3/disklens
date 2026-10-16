export const SHADES = ["50", "100", "200", "300", "400", "500", "600", "700", "800", "900", "950"] as const;
export type Shade = (typeof SHADES)[number];
export type Scale = Record<Shade, string>;

/**
 * HSL lightness of each shade. The app's palettes keep the 500 color's hue and
 * saturation (rounded to whole degrees and percent) and step the lightness.
 */
const LIGHTNESS: Record<Exclude<Shade, "500">, number> = {
  "50": 0.95,
  "100": 0.9,
  "200": 0.8,
  "300": 0.7,
  "400": 0.6,
  "600": 0.4,
  "700": 0.3,
  "800": 0.2,
  "900": 0.1,
  "950": 0.07,
};

type Rgb = [number, number, number];

/** Parses `rgb(r g b)`, `rgb(r, g, b)` or `#rrggbb`. */
export function parseColor(color: string): Rgb {
  const hex = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(color.trim());
  if (hex) return [parseInt(hex[1], 16), parseInt(hex[2], 16), parseInt(hex[3], 16)];
  const rgb = /^rgb\(\s*(\d+)[\s,]+(\d+)[\s,]+(\d+)/i.exec(color.trim());
  if (rgb) return [Number(rgb[1]), Number(rgb[2]), Number(rgb[3])];
  throw new Error(`Unsupported color: ${color}`);
}

const format = ([r, g, b]: Rgb) => `rgb(${r} ${g} ${b})`;

function toHsl([r, g, b]: Rgb): [number, number] {
  const [rn, gn, bn] = [r / 255, g / 255, b / 255];
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const d = max - min;
  if (d === 0) return [0, 0];
  const l = (max + min) / 2;
  const s = d / (1 - Math.abs(2 * l - 1));
  let h = max === rn ? ((gn - bn) / d) % 6 : max === gn ? (bn - rn) / d + 2 : (rn - gn) / d + 4;
  h *= 60;
  return [h < 0 ? h + 360 : h, s];
}

function fromHsl(h: number, s: number, l: number): Rgb {
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = l - c / 2;
  const [r, g, b] =
    h < 60 ? [c, x, 0] : h < 120 ? [x, c, 0] : h < 180 ? [0, c, x] : h < 240 ? [0, x, c] : h < 300 ? [x, 0, c] : [c, 0, x];
  return [r, g, b].map((v) => Math.round((v + m) * 255)) as Rgb;
}

/** A full 50–950 family from its 500 color, built the same way as the app's palettes. */
export function scale(base: string): Scale {
  const rgb = parseColor(base);
  const [hue, sat] = toHsl(rgb);
  const [h, s] = [Math.round(hue), Math.round(sat * 100) / 100];
  const out = { "500": format(rgb) } as Scale;
  for (const [shade, l] of Object.entries(LIGHTNESS) as [Exclude<Shade, "500">, number][]) {
    out[shade] = format(fromHsl(h, s, l));
  }
  return out;
}

/** The same color at `opacity`. */
export function alpha(color: string, opacity: number): string {
  const [r, g, b] = parseColor(color);
  return `rgb(${r} ${g} ${b} / ${opacity})`;
}

/** The color moved toward white by `amount` (0–1). */
export function mixWhite(color: string, amount: number): string {
  return format(parseColor(color).map((c) => Math.round(c + amount * (255 - c))) as Rgb);
}
