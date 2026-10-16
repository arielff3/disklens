import type { Category, PlaceKey } from "../types";
import { INK, LAGOON, SAND, type Palette } from "./palettes";
import type { Shade } from "./scale";

export const MODES = ["dark", "light"] as const;
export type Mode = (typeof MODES)[number];
export type ThemeId = "sand" | "ink" | "lagoon";

/** A family and shade in the theme's palette. */
export type Ref = readonly [family: string, shade: Shade];

export interface ThemeDef {
  id: ThemeId;
  palette: Palette;
  /** Text, borders and the dark-mode backgrounds. */
  neutral: string;
  /** Dark-mode sidebar. */
  sidebar: string;
  /** Light-mode background and sidebar. */
  tint: string;
  /** Dark-mode secondary text. */
  muted: string;
  accent: string;
  safe: string;
  caution: string;
  /** Size bars in lists. */
  barFill: string;
  categories: Record<Mode, Record<Category, Ref>>;
  /** Treemap tile fills, assigned by rank. */
  tiles: Record<Mode, readonly Ref[]>;
  places: Record<PlaceKey, Ref>;
  drive: Ref;
}

export const TILE_COUNT = 8;

const sand: ThemeDef = {
  id: "sand",
  palette: SAND,
  neutral: "sand",
  sidebar: "sand",
  tint: "sand",
  muted: "sand",
  accent: "ember",
  safe: "leaf",
  caution: "amber",
  barFill: "sand",
  categories: {
    dark: {
      video: ["coral", "400"],
      images: ["lavender", "400"],
      audio: ["amber", "400"],
      archives: ["tan", "400"],
      code: ["leaf", "400"],
      documents: ["sky", "400"],
      apps: ["rose", "400"],
      dataCaches: ["slate", "400"],
      other: ["sand", "400"],
    },
    light: {
      video: ["coral", "500"],
      images: ["lavender", "500"],
      audio: ["amber", "500"],
      archives: ["tan", "500"],
      code: ["leaf", "500"],
      documents: ["sky", "500"],
      apps: ["rose", "500"],
      dataCaches: ["slate", "500"],
      other: ["sand", "300"],
    },
  },
  tiles: {
    dark: [["coral", "700"], ["sky", "700"], ["mint", "700"], ["rose", "700"], ["lavender", "700"], ["amber", "700"], ["leaf", "700"], ["tan", "700"]],
    light: [["coral", "200"], ["sky", "200"], ["mint", "200"], ["rose", "200"], ["lavender", "200"], ["amber", "200"], ["leaf", "200"], ["tan", "200"]],
  },
  places: {
    home: ["lavender", "500"],
    downloads: ["sky", "500"],
    documents: ["amber", "500"],
    desktop: ["rose", "500"],
    pictures: ["leaf", "500"],
    videos: ["coral", "500"],
    appdata: ["mint", "500"],
    programFiles: ["slate", "500"],
  },
  drive: ["ember", "400"],
};

const ink: ThemeDef = {
  id: "ink",
  palette: INK,
  neutral: "ink-black",
  sidebar: "dark-teal",
  tint: "pearl-aqua",
  muted: "pearl-aqua",
  accent: "golden-orange",
  safe: "pearl-aqua",
  caution: "golden-orange",
  barFill: "dark-cyan",
  categories: {
    dark: {
      video: ["rusty-spice", "400"],
      images: ["golden-orange", "400"],
      audio: ["vanilla-custard", "300"],
      archives: ["brown-red", "300"],
      code: ["pearl-aqua", "400"],
      documents: ["ink-black", "400"],
      apps: ["oxidized-iron", "400"],
      dataCaches: ["dark-cyan", "600"],
      other: ["pearl-aqua", "200"],
    },
    light: {
      video: ["rusty-spice", "500"],
      images: ["golden-orange", "600"],
      audio: ["vanilla-custard", "600"],
      archives: ["brown-red", "400"],
      code: ["pearl-aqua", "600"],
      documents: ["ink-black", "600"],
      apps: ["oxidized-iron", "500"],
      dataCaches: ["dark-cyan", "700"],
      other: ["pearl-aqua", "300"],
    },
  },
  tiles: {
    dark: [
      ["ink-black", "700"],
      ["burnt-caramel", "700"],
      ["pearl-aqua", "700"],
      ["brown-red", "700"],
      ["dark-cyan", "700"],
      ["golden-orange", "700"],
      ["dark-teal", "800"],
      ["rusty-spice", "700"],
    ],
    light: [
      ["ink-black", "200"],
      ["burnt-caramel", "200"],
      ["pearl-aqua", "200"],
      ["brown-red", "200"],
      ["dark-cyan", "200"],
      ["golden-orange", "200"],
      ["dark-teal", "200"],
      ["rusty-spice", "200"],
    ],
  },
  places: {
    home: ["ink-black", "600"],
    downloads: ["dark-teal", "600"],
    documents: ["vanilla-custard", "600"],
    desktop: ["brown-red", "500"],
    pictures: ["pearl-aqua", "600"],
    videos: ["rusty-spice", "600"],
    appdata: ["dark-cyan", "700"],
    programFiles: ["ink-black", "800"],
  },
  drive: ["burnt-caramel", "600"],
};

const lagoon: ThemeDef = {
  id: "lagoon",
  palette: LAGOON,
  neutral: "cerulean",
  sidebar: "tropical-teal",
  tint: "light-yellow",
  muted: "cerulean",
  accent: "soft-apricot",
  safe: "tropical-teal",
  caution: "light-yellow",
  barFill: "tropical-teal",
  categories: {
    dark: {
      video: ["vibrant-coral", "400"],
      images: ["soft-apricot", "400"],
      audio: ["light-yellow", "300"],
      archives: ["soft-apricot", "200"],
      code: ["tropical-teal", "400"],
      documents: ["cerulean", "400"],
      apps: ["vibrant-coral", "300"],
      dataCaches: ["tropical-teal", "600"],
      other: ["cerulean", "200"],
    },
    light: {
      video: ["vibrant-coral", "500"],
      images: ["soft-apricot", "500"],
      audio: ["light-yellow", "700"],
      archives: ["soft-apricot", "300"],
      code: ["tropical-teal", "700"],
      documents: ["cerulean", "600"],
      apps: ["vibrant-coral", "300"],
      dataCaches: ["tropical-teal", "800"],
      other: ["cerulean", "300"],
    },
  },
  tiles: {
    dark: [
      ["cerulean", "700"],
      ["soft-apricot", "700"],
      ["tropical-teal", "700"],
      ["vibrant-coral", "700"],
      ["light-yellow", "700"],
      ["cerulean", "800"],
      ["soft-apricot", "800"],
      ["tropical-teal", "800"],
    ],
    light: [
      ["cerulean", "200"],
      ["soft-apricot", "200"],
      ["tropical-teal", "200"],
      ["vibrant-coral", "200"],
      ["light-yellow", "200"],
      ["cerulean", "100"],
      ["soft-apricot", "100"],
      ["tropical-teal", "100"],
    ],
  },
  places: {
    home: ["cerulean", "600"],
    downloads: ["tropical-teal", "700"],
    documents: ["light-yellow", "700"],
    desktop: ["vibrant-coral", "500"],
    pictures: ["tropical-teal", "600"],
    videos: ["soft-apricot", "600"],
    appdata: ["cerulean", "700"],
    programFiles: ["cerulean", "800"],
  },
  drive: ["soft-apricot", "600"],
};

export const THEMES: Record<ThemeId, ThemeDef> = { sand, ink, lagoon };
export const THEME_ORDER: ThemeId[] = ["sand", "ink", "lagoon"];
