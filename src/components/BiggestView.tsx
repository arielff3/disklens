import { useEffect, useState, type MouseEvent } from "react";
import { api } from "../api";
import { NO_CHIPS, toFilter, type Chips, type SizeChip } from "../biggest";
import { S } from "../strings";
import { CATEGORY_COLORS } from "../theme";
import type { Category, Hit, SearchResult } from "../types";
import { ResultList } from "./ResultList";
import { ResultsHeader } from "./ResultsHeader";

interface Props {
  /** Bumped whenever the index changes, to refetch. */
  version: number;
  onOpen: (hit: Hit) => void;
  onOpenFile: (hit: Hit) => void;
  onMenu: (e: MouseEvent, hit: Hit) => void;
}

const SIZES: SizeChip[] = ["all", "100mb", "1gb"];

export function BiggestView({ version, onOpen, onOpenFile, onMenu }: Props) {
  const [chips, setChips] = useState<Chips>(NO_CHIPS);
  const [result, setResult] = useState<SearchResult | null>(null);

  useEffect(() => {
    let stale = false;
    api
      .biggest(toFilter(chips))
      .then((r) => !stale && setResult(r))
      .catch(console.error);
    return () => {
      stale = true;
    };
  }, [chips, version]);

  const set = (change: Partial<Chips>) => setChips((c) => ({ ...c, ...change }));
  const categories = Object.keys(S.categories) as Category[];

  return (
    <div className="results-view">
      <div className="chips">
        {SIZES.map((size) => (
          <button key={size} className={`chip${chips.size === size ? " on" : ""}`} onClick={() => set({ size })}>
            {S.chips[size]}
          </button>
        ))}
        <span className="chip-sep" />
        <button className={`chip${chips.old ? " on" : ""}`} onClick={() => set({ old: !chips.old })}>
          {S.chips.old}
        </button>
        <span className="chip-sep" />
        {categories.map((c) => (
          <button
            key={c}
            className={`chip${chips.category === c ? " on" : ""}`}
            onClick={() => set({ category: chips.category === c ? null : c })}
          >
            <span className="dot" style={{ background: CATEGORY_COLORS[c] }} />
            {S.categories[c]}
          </button>
        ))}
      </div>
      <ResultsHeader title={S.biggestTitle} result={result} />
      {result && <ResultList hits={result.results} onOpen={onOpen} onOpenFile={onOpenFile} onMenu={onMenu} />}
    </div>
  );
}
