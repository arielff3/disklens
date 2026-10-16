import { S } from "../strings";
import type { SearchResult } from "../types";

export function ResultsHeader({ title, result }: { title: string; result: SearchResult | null }) {
  return (
    <div className="results-head">
      <h3>{title}</h3>
      {result && (
        <span className="muted small">
          {S.resultCount(result.total, result.elapsedMs)}
          {result.total > result.results.length && ` · ${S.showingLargest(result.results.length)}`}
        </span>
      )}
    </div>
  );
}
