import { useEffect, useState, type Dispatch, type SetStateAction } from "react";

const store = new Map<string, unknown>();

/**
 * Like `useState`, but the value outlives the component. Switching tabs
 * unmounts a view; a delete or uninstall it started keeps running in the
 * background, and the view must pick it up again when you come back.
 */
export function useSessionState<T>(key: string, initial: T): [T, Dispatch<SetStateAction<T>>] {
  const [value, setValue] = useState<T>(() => (store.has(key) ? (store.get(key) as T) : initial));
  useEffect(() => {
    store.set(key, value);
  }, [key, value]);
  return [value, setValue];
}

/** Forgets a value right away, for components about to unmount. */
export function clearSessionState(key: string): void {
  store.delete(key);
}
