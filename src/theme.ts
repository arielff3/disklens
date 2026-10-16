import type { Category, PlaceKey } from "./types";

// Colors come from the active theme as CSS variables (see src/themes); these
// are references to them, usable anywhere CSS accepts a color.

const cssVar = (name: string) => `var(--${name})`;

export const CATEGORY_COLORS: Record<Category, string> = {
  video: cssVar("cat-video"),
  images: cssVar("cat-images"),
  audio: cssVar("cat-audio"),
  archives: cssVar("cat-archives"),
  code: cssVar("cat-code"),
  documents: cssVar("cat-documents"),
  apps: cssVar("cat-apps"),
  dataCaches: cssVar("cat-dataCaches"),
  other: cssVar("cat-other"),
};

export const PLACE_COLORS: Record<PlaceKey, string> = {
  home: cssVar("place-home"),
  downloads: cssVar("place-downloads"),
  documents: cssVar("place-documents"),
  desktop: cssVar("place-desktop"),
  pictures: cssVar("place-pictures"),
  videos: cssVar("place-videos"),
  appdata: cssVar("place-appdata"),
  programFiles: cssVar("place-programFiles"),
};

export const DRIVE_COLOR = cssVar("drive");

/** Colors for the small inline SVG icons (use them in `style`, not attributes). */
export const ICON_COLORS = {
  logoBack: cssVar("logo-back"),
  logoFace: cssVar("logo-face"),
  logoSlice: cssVar("logo-slice"),
  folderBack: cssVar("folder-back"),
  folderFront: cssVar("folder-front"),
  filePage: cssVar("file-page"),
  fileEdge: cssVar("file-edge"),
};
