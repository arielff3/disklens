import type { MouseEvent } from "react";
import { S } from "../strings";
import type { Hit, SearchResult } from "../types";
import { ResultList } from "./ResultList";
import { ResultsHeader } from "./ResultsHeader";

interface Props {
  result: SearchResult | null;
  error: string | null;
  onOpen: (hit: Hit) => void;
  onOpenFile: (hit: Hit) => void;
  onMenu: (e: MouseEvent, hit: Hit) => void;
}

export function SearchView({ result, error, onOpen, onOpenFile, onMenu }: Props) {
  return (
    <div className="results-view">
      <ResultsHeader title={S.searchResults} result={error ? null : result} />
      {error ? (
        <div className="card notice-card">
          <div className="notice">{error}</div>
          <p className="muted small">{S.searchHint}</p>
        </div>
      ) : (
        result && <ResultList hits={result.results} onOpen={onOpen} onOpenFile={onOpenFile} onMenu={onMenu} />
      )}
    </div>
  );
}
