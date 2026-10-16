import type { BiggestFilter, Category } from "./types";

export type SizeChip = "all" | "100mb" | "1gb";

export interface Chips {
  size: SizeChip;
  old: boolean;
  category: Category | null;
}

export const NO_CHIPS: Chips = { size: "all", old: false, category: null };

const MIN_SIZE: Record<SizeChip, number | null> = { all: null, "100mb": 100 * 1024 ** 2, "1gb": 1024 ** 3 };

/** "Older than 1 year". */
const OLD_DAYS = 365;

export function toFilter(chips: Chips): BiggestFilter {
  return {
    minSize: MIN_SIZE[chips.size],
    olderThanDays: chips.old ? OLD_DAYS : null,
    category: chips.category,
  };
}
