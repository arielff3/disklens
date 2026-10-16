import { S } from "./strings";

const UNITS = ["B", "KB", "MB", "GB", "TB", "PB"];

/** Size split into number and unit, using 1024-based units like Windows Explorer. */
export function splitBytes(bytes: number): { value: string; unit: string } {
  if (bytes < 1024) return { value: String(Math.round(bytes)), unit: "B" };
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit++;
  }
  const digits = value >= 100 ? 1 : 2;
  return { value: trimZeros(value.toFixed(digits)), unit: UNITS[unit] };
}

export function formatBytes(bytes: number): string {
  const { value, unit } = splitBytes(bytes);
  return `${value} ${unit}`;
}

export function formatCount(n: number): string {
  return n.toLocaleString("en-US");
}

export function formatFiles(n: number): string {
  return `${formatCount(n)} ${n === 1 ? "file" : "files"}`;
}

export function percent(part: number, total: number): number {
  return total > 0 ? part / total : 0;
}

function trimZeros(s: string): string {
  return s.includes(".") ? s.replace(/\.?0+$/, "") : s;
}

const DAY = 86_400;

/** "today", "5 days ago", "3 years ago". Empty for unknown (0) times. */
export function formatAge(modified: number, now: number): string {
  if (modified <= 0) return "";
  const days = Math.floor(Math.max(0, now - modified) / DAY);
  if (days < 1) return S.age.today;
  if (days < 2) return S.age.yesterday;
  if (days < 30) return S.age.ago(days, "day");
  if (days < 365) return S.age.ago(Math.floor(days / 30), "month");
  return S.age.ago(Math.floor(days / 365), "year");
}
