import { S } from "../strings";

/** A failed load, with its message and a way to try again (instead of an endless spinner). */
export function LoadError({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div className="card sysdata-admin">
      <div className="notice">
        {S.loadFailed} {message}
      </div>
      <button className="btn" onClick={onRetry}>
        {S.tryAgain}
      </button>
    </div>
  );
}
