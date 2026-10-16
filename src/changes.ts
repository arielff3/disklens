import { formatBytes } from "./format";
import { isWithin } from "./paths";

/** "+1.5 GB", "−200 MB", or "0 B". */
export function formatDelta(bytes: number): string {
  if (bytes === 0) return "0 B";
  return `${bytes > 0 ? "+" : "−"}${formatBytes(Math.abs(bytes))}`;
}

/** `path` without the scanned root in front, when it is inside it. */
export function relativeTo(path: string, root: string): string {
  if (!isWithin(path, root)) return path;
  const prefix = root.endsWith("\\") ? root : `${root}\\`;
  return path.slice(prefix.length) || path;
}

export function formatWhen(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleString("en-US", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}
