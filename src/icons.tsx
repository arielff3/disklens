import { CATEGORY_COLORS, ICON_COLORS, PLACE_COLORS } from "./theme";
import type { Category, PlaceKey } from "./types";

export function Logo() {
  return (
    <svg viewBox="0 0 1024 1024" aria-hidden>
      <rect x="64" y="64" width="896" height="896" rx="200" style={{ fill: ICON_COLORS.logoBack }} />
      <circle cx="512" cy="512" r="270" style={{ fill: ICON_COLORS.logoFace }} />
      <path d="M512 512 L512 242 A270 270 0 0 1 768.8 595.4 Z" style={{ fill: ICON_COLORS.logoSlice }} />
      <circle cx="512" cy="512" r="60" style={{ fill: ICON_COLORS.logoBack }} />
    </svg>
  );
}

const stroke = { fill: "none", stroke: "currentColor", strokeWidth: 1.6, strokeLinecap: "round", strokeLinejoin: "round" } as const;

export function BackIcon() {
  return (
    <svg viewBox="0 0 16 16" {...stroke}>
      <path d="M10 3 5 8l5 5" />
    </svg>
  );
}

export function RefreshIcon() {
  return (
    <svg viewBox="0 0 16 16" {...stroke}>
      <path d="M13 8a5 5 0 1 1-1.5-3.6" />
      <path d="M13 2.5V5h-2.5" />
    </svg>
  );
}

export function SearchIcon() {
  return (
    <svg viewBox="0 0 16 16" {...stroke}>
      <circle cx="7" cy="7" r="4.5" />
      <path d="m10.5 10.5 3 3" />
    </svg>
  );
}

export function CloseIcon() {
  return (
    <svg viewBox="0 0 16 16" {...stroke}>
      <path d="m4.5 4.5 7 7M11.5 4.5l-7 7" />
    </svg>
  );
}

export function DriveIcon() {
  return (
    <svg viewBox="0 0 16 16" {...stroke} stroke="white">
      <rect x="2" y="4" width="12" height="8" rx="2" />
      <path d="M10.5 8h1" />
    </svg>
  );
}

const PLACE_GLYPHS: Record<PlaceKey, string> = {
  home: "M3 7.5 8 3.5l5 4V13H3z",
  downloads: "M8 3v7M5 7.5 8 10.5l3-3M4 13h8",
  documents: "M5 2.5h4l2.5 2.5v8.5h-6.5z",
  desktop: "M2.5 3.5h11v7h-11zM6 13h4",
  pictures: "M2.5 3.5h11v9h-11zM3 11l3.5-3.5L9 10l2-2 2.5 2.5",
  videos: "M5.5 4v8l6-4z",
  appdata: "M8 5.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5zM8 2v2M8 12v2M2 8h2M12 8h2",
  programFiles: "M3 3h4v4H3zM9 3h4v4H9zM3 9h4v4H3zM9 9h4v4H9z",
};

export function PlaceIcon({ place }: { place: PlaceKey }) {
  return (
    <span className="side-icon" style={{ background: PLACE_COLORS[place] }}>
      <svg viewBox="0 0 16 16" {...stroke} stroke="white" strokeWidth={1.5}>
        <path d={PLACE_GLYPHS[place]} />
      </svg>
    </span>
  );
}

export function FolderIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden>
      <path d="M3 6.5A1.5 1.5 0 0 1 4.5 5h4.6l2 2h8.4A1.5 1.5 0 0 1 21 8.5v9a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 17.5z" style={{ fill: ICON_COLORS.folderBack }} />
      <path d="M3 9h18v8.5a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 17.5z" style={{ fill: ICON_COLORS.folderFront }} />
    </svg>
  );
}

export function FileIcon({ category }: { category: Category }) {
  return (
    <svg viewBox="0 0 24 24" aria-hidden>
      <path d="M6 3h8l5 5v12a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z" style={{ fill: ICON_COLORS.filePage, stroke: ICON_COLORS.fileEdge }} />
      <path d="M14 3v5h5" fill="none" style={{ stroke: ICON_COLORS.fileEdge }} />
      <rect x="7.5" y="13" width="9" height="4" rx="1" style={{ fill: CATEGORY_COLORS[category] }} />
    </svg>
  );
}

export function ExploreIcon() {
  return (
    <svg viewBox="0 0 16 16" {...stroke}>
      <path d="M2.5 2.5h11v11h-11zM2.5 8.5h6M8.5 2.5v11" />
    </svg>
  );
}

export function RevealIcon() {
  return (
    <svg viewBox="0 0 16 16" {...stroke}>
      <path d="M2 4.5h4l1.5 1.5H14v6.5H2z" />
    </svg>
  );
}

export function SunIcon() {
  return (
    <svg viewBox="0 0 16 16" {...stroke}>
      <circle cx="8" cy="8" r="3" />
      <path d="M8 1.5v1.5M8 13v1.5M1.5 8H3M13 8h1.5M3.4 3.4l1 1M11.6 11.6l1 1M3.4 12.6l1-1M11.6 4.4l1-1" />
    </svg>
  );
}

export function MoonIcon() {
  return (
    <svg viewBox="0 0 16 16" {...stroke}>
      <path d="M13.5 9.5A5.5 5.5 0 1 1 6.5 2.5a4.5 4.5 0 0 0 7 7z" />
    </svg>
  );
}

export function CheckIcon() {
  return (
    <svg viewBox="0 0 16 16" {...stroke}>
      <path d="M3.5 8.5 6.5 11.5 12.5 4.5" />
    </svg>
  );
}
