import type { SVGProps } from "react";

// Stroke icons drawn on a 24px grid, in the same style as the app's own.
const base = {
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.7,
  strokeLinecap: "round",
  strokeLinejoin: "round",
} as const;

type IconProps = SVGProps<SVGSVGElement>;

export const Logo = (props: IconProps) => (
  <svg viewBox="0 0 1024 1024" aria-hidden="true" {...props}>
    <rect x="64" y="64" width="896" height="896" rx="200" fill="#2b2522" />
    <circle cx="512" cy="512" r="270" fill="#f7f2eb" />
    <path d="M512 512 L512 242 A270 270 0 0 1 768.8 595.4 Z" fill="#f26b2a" />
    <circle cx="512" cy="512" r="60" fill="#2b2522" />
  </svg>
);

export const WindowsIcon = (props: IconProps) => (
  <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" {...props}>
    <path d="M3 5.5 10.5 4.4v7.1H3zM11.5 4.3 21 3v8.5h-9.5zM3 12.5h7.5v7.1L3 18.5zM11.5 12.5H21V21l-9.5-1.3z" />
  </svg>
);

export const GitHubIcon = (props: IconProps) => (
  <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" {...props}>
    <path d="M12 2a10 10 0 0 0-3.16 19.49c.5.09.68-.22.68-.48v-1.7c-2.78.6-3.37-1.34-3.37-1.34-.45-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.9 1.52 2.34 1.08 2.91.83.09-.65.35-1.09.63-1.34-2.22-.25-4.55-1.11-4.55-4.94 0-1.09.39-1.98 1.03-2.68-.1-.25-.45-1.27.1-2.64 0 0 .84-.27 2.75 1.02a9.5 9.5 0 0 1 5 0c1.91-1.29 2.75-1.02 2.75-1.02.55 1.37.2 2.39.1 2.64.64.7 1.03 1.59 1.03 2.68 0 3.84-2.34 4.68-4.57 4.93.36.31.68.92.68 1.85v2.74c0 .27.18.58.69.48A10 10 0 0 0 12 2z" />
  </svg>
);

export const ExploreIcon = (props: IconProps) => (
  <svg {...base} aria-hidden="true" {...props}>
    <rect x="3" y="3" width="18" height="18" rx="3" />
    <path d="M12 3v18M12 12h9M16.5 12v9" />
  </svg>
);

export const ShieldIcon = (props: IconProps) => (
  <svg {...base} aria-hidden="true" {...props}>
    <path d="M12 3 4.5 6v5.5c0 4.5 3.2 8.2 7.5 9.5 4.3-1.3 7.5-5 7.5-9.5V6z" />
    <path d="m9 12 2 2 4-4" />
  </svg>
);

export const CodeIcon = (props: IconProps) => (
  <svg {...base} aria-hidden="true" {...props}>
    <path d="M3 7.5A1.5 1.5 0 0 1 4.5 6h4.6l2 2h8.4A1.5 1.5 0 0 1 21 9.5v8a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 17.5z" />
    <path d="m10 11.5-2 2 2 2M14 11.5l2 2-2 2" />
  </svg>
);

export const ChartIcon = (props: IconProps) => (
  <svg {...base} aria-hidden="true" {...props}>
    <path d="M4 19h16M4 15l5-5 4 3 7-7" />
    <path d="M15 6h5v5" />
  </svg>
);

export const SparkIcon = (props: IconProps) => (
  <svg {...base} aria-hidden="true" {...props}>
    <path d="M12 3v4M12 17v4M3 12h4M17 12h4M6 6l2.5 2.5M15.5 15.5 18 18M18 6l-2.5 2.5M8.5 15.5 6 18" />
  </svg>
);

export const AppsIcon = (props: IconProps) => (
  <svg {...base} aria-hidden="true" {...props}>
    <rect x="3.5" y="3.5" width="7" height="7" rx="1.5" />
    <rect x="13.5" y="3.5" width="7" height="7" rx="1.5" />
    <rect x="3.5" y="13.5" width="7" height="7" rx="1.5" />
    <path d="M17 13.5v7M13.5 17h7" />
  </svg>
);

export const ArrowIcon = (props: IconProps) => (
  <svg {...base} aria-hidden="true" {...props}>
    <path d="M5 12h14M13 6l6 6-6 6" />
  </svg>
);
