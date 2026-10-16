import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import { S } from "../strings";
import { useSessionState } from "../sessionState";
import type { ToolId, ToolStatus, ToolView } from "../types";

const POLL_MS = 1000;

interface Props {
  /** A tool finished: sizes may have changed. */
  onChanged: () => void;
}

/** Windows' own space-saving tools, at the bottom of System Data. */
export function WindowsTools({ onChanged }: Props) {
  const [tools, setTools] = useState<ToolView[]>([]);
  const [confirming, setConfirming] = useState<ToolId | null>(null);
  const [statuses, setStatuses] = useSessionState<Partial<Record<ToolId, ToolStatus>>>("tools.statuses", {});

  const load = useCallback(() => {
    api.toolsList().then(setTools).catch(console.error);
  }, []);
  useEffect(load, [load]);

  const running = (Object.keys(statuses) as ToolId[]).filter((id) => statuses[id]?.phase === "running");
  useEffect(() => {
    if (running.length === 0) return;
    const timer = window.setInterval(() => {
      for (const id of running) {
        api
          .toolStatus(id)
          .then((status) => {
            if (!status || status.phase === "running") return;
            setStatuses((s) => ({ ...s, [id]: status }));
            load();
            onChanged();
          })
          .catch(console.error);
      }
    }, POLL_MS);
    return () => window.clearInterval(timer);
  }, [running.join(","), load, onChanged]); // eslint-disable-line react-hooks/exhaustive-deps

  const run = (tool: ToolView) => {
    setConfirming(null);
    if (tool.elevated) setStatuses((s) => ({ ...s, [tool.id]: { phase: "running", message: null } }));
    api.toolRun(tool.id).catch((e) => setStatuses((s) => ({ ...s, [tool.id]: { phase: "failed", message: String(e) } })));
  };

  return (
    <section className="card group-card tools-card">
      <h4>{S.windowsTools}</h4>
      <p className="muted small tools-body">{S.windowsToolsBody}</p>
      <div className="sys-rows">
        {tools.map((tool) => {
          const text = S.tools[tool.id];
          const status = statuses[tool.id];
          return (
            <div key={tool.id} className="sys-row">
              <div className="sys-title">
                <span className="name">{text.title}</span>
              </div>
              <div className="sys-size">
                <ToolAction
                  tool={tool}
                  status={status}
                  confirming={confirming === tool.id}
                  onAsk={() => (tool.elevated ? setConfirming(tool.id) : run(tool))}
                  onRun={() => run(tool)}
                  onCancel={() => setConfirming(null)}
                />
              </div>
              <div className="sys-explain">{text.explain}</div>
              {status?.phase === "failed" && status.message && <div className="sys-note">{status.message}</div>}
            </div>
          );
        })}
      </div>
    </section>
  );
}

interface ActionProps {
  tool: ToolView;
  status: ToolStatus | undefined;
  confirming: boolean;
  onAsk: () => void;
  onRun: () => void;
  onCancel: () => void;
}

function ToolAction({ tool, status, confirming, onAsk, onRun, onCancel }: ActionProps) {
  if (status?.phase === "running") {
    return (
      <span className="pill">
        <span className="mini-spinner" />
        {S.toolRunning}
      </span>
    );
  }
  if (!tool.available) return <span className="verdict safeToClear">{tool.on ? S.toolAlreadyOn : S.toolNotNeeded}</span>;
  if (status?.phase === "done" && tool.elevated) return <span className="verdict safeToClear">{S.toolDone}</span>;
  if (confirming) {
    return (
      <span className="tool-confirm">
        <span className="muted small">{S.toolConfirm}</span>
        <button className="btn" onClick={onCancel}>
          {S.cancel}
        </button>
        <button className="btn primary" onClick={onRun}>
          {S.run}
        </button>
      </span>
    );
  }
  return (
    <button className="btn" onClick={onAsk}>
      {S.tools[tool.id].button}
    </button>
  );
}
