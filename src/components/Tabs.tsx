import { S } from "../strings";

export type TabKey = keyof typeof S.tabs;

/** Tabs that exist in this milestone; the rest are shown disabled. */
const ENABLED = new Set<TabKey>(["explore", "systemData", "biggest", "changes", "projects", "cleanup", "apps"]);

interface Props {
  /** None while search results are shown. */
  active: TabKey | null;
  onChange: (tab: TabKey) => void;
}

export function Tabs({ active, onChange }: Props) {
  const keys = Object.keys(S.tabs) as TabKey[];
  return (
    <div className="tabs" role="tablist">
      {keys.map((key) => {
        const enabled = ENABLED.has(key);
        return (
          <button
            key={key}
            role="tab"
            aria-selected={active === key}
            className={`tab${active === key ? " active" : ""}`}
            disabled={!enabled}
            title={enabled ? undefined : S.comingSoon}
            onClick={() => onChange(key)}
          >
            {S.tabs[key]}
          </button>
        );
      })}
    </div>
  );
}
