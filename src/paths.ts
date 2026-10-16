export function joinPath(parent: string, name: string): string {
  return parent.endsWith("\\") ? parent + name : `${parent}\\${name}`;
}

export function samePath(a: string, b: string): boolean {
  return trimSep(a).toLowerCase() === trimSep(b).toLowerCase();
}

/** True if `path` is `root` or inside it. */
export function isWithin(path: string, root: string): boolean {
  const p = trimSep(path).toLowerCase();
  const r = trimSep(root).toLowerCase();
  return p === r || p.startsWith(r.endsWith("\\") ? r : `${r}\\`);
}

/** `C:` for a drive root, otherwise the last path component. */
export function displayName(path: string): string {
  const trimmed = trimSep(path);
  if (/^[a-z]:\\?$/i.test(trimmed)) return trimmed.slice(0, 2);
  return trimmed.split("\\").pop() || trimmed;
}

/** Drops trailing separators, but keeps the one in a drive root like `C:\`. */
function trimSep(path: string): string {
  let p = path.replace(/\//g, "\\");
  while (p.length > 3 && p.endsWith("\\")) p = p.slice(0, -1);
  return p;
}

/** The folder containing `path`; a drive root is its own parent. */
export function parentPath(path: string): string {
  const p = trimSep(path);
  const i = p.lastIndexOf("\\");
  if (i < 0 || p.length <= 3) return p;
  return i === 2 ? p.slice(0, 3) : p.slice(0, i);
}
