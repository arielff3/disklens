"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import ClickSpark from "@/components/reactbits/ClickSpark";
import Counter from "@/components/reactbits/Counter";
import { squarify, type Rect } from "@/lib/squarify";

type Verdict = "safe" | "look" | "managed";

type Item = { id: string; name: string; where: string; gb: number; verdict: Verdict; note: string };

// A made-up but typical C: drive, using the verdicts Disklens gives.
const ITEMS: Item[] = [
  { id: "docker", name: "Docker disk", where: "AppData\\Local\\Docker", gb: 33.1, verdict: "look", note: "Docker's virtual disk only grows. Disklens compacts it from System Data, without deleting your images." },
  { id: "winsxs", name: "WinSxS", where: "Windows\\WinSxS", gb: 9.8, verdict: "managed", note: "WinSxS is managed by Windows. Disklens explains it, and points you to DISM, but never deletes it." },
  { id: "node", name: "node_modules", where: "14 old projects", gb: 8.7, verdict: "safe", note: "" },
  { id: "pagefile", name: "pagefile.sys", where: "C:\\", gb: 8, verdict: "managed", note: "The page file is Windows' virtual memory. Windows sizes it, so Disklens leaves it alone." },
  { id: "hiberfil", name: "hiberfil.sys", where: "C:\\", gb: 6.4, verdict: "look", note: "The hibernation file goes away if you turn hibernation off, which Disklens can do for you." },
  { id: "downloads", name: "Downloads", where: "Users\\you\\Downloads", gb: 5.4, verdict: "look", note: "Old installers and ISOs, probably. Your files, so Disklens shows them and lets you decide." },
  { id: "target", name: "Rust target", where: "3 old projects", gb: 4.2, verdict: "safe", note: "" },
  { id: "npm", name: "npm cache", where: "AppData\\Local\\npm-cache", gb: 3.4, verdict: "safe", note: "" },
  { id: "temp", name: "Temp", where: "AppData\\Local\\Temp", gb: 2.7, verdict: "safe", note: "" },
  { id: "bin", name: "Recycle Bin", where: "$Recycle.Bin", gb: 2.1, verdict: "look", note: "Deleted files still take space until you empty the bin." },
  { id: "updates", name: "Update downloads", where: "SoftwareDistribution", gb: 1.2, verdict: "safe", note: "" },
  { id: "chrome", name: "Chrome cache", where: "AppData\\Local\\Google", gb: 0.9, verdict: "safe", note: "" },
];

const TILES = ["#f8b6a0", "#a5d0f3", "#b6e2de", "#efa9ca", "#ada0f8", "#f8d9a0", "#c5e5b3", "#e9ccaf"];
const VERDICT: Record<Verdict, { label: string; dot: string; chip: string }> = {
  safe: { label: "Safe to clear", dot: "bg-[#5c9e3a]", chip: "bg-[rgb(127_198_90/0.22)] text-leaf" },
  look: { label: "Look first", dot: "bg-[#e0a22a]", chip: "bg-[rgb(242_179_61/0.24)] text-amber" },
  managed: { label: "Managed by Windows", dot: "bg-[#9e9488]", chip: "bg-[rgb(158_148_136/0.2)] text-[#5f5750]" },
};

const DRIVE_GB = 476;
const START_FREE = 9.4;
const SAFE_TOTAL = ITEMS.filter((i) => i.verdict === "safe").reduce((a, i) => a + i.gb, 0);

export default function DiskMap() {
  const box = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ w: 4, h: 3 });
  const [cleared, setCleared] = useState<Set<string>>(new Set());
  const [ready, setReady] = useState(false);
  const [settled, setSettled] = useState(false);
  const [collapsed, setCollapsed] = useState<Record<string, Rect>>({});
  const [status, setStatus] = useState<{ text: string; tone: Verdict } | null>(null);
  const [shake, setShake] = useState<string | null>(null);

  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const measure = () => setSize({ w: el.clientWidth, h: el.clientHeight });
    // Measure right away too: ResizeObserver only reports on a rendered frame.
    const first = setTimeout(measure, 0);
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    const t = setTimeout(() => setReady(true), 120);
    const u = setTimeout(() => setSettled(true), 1400);
    return () => {
      clearTimeout(first);
      ro.disconnect();
      clearTimeout(t);
      clearTimeout(u);
    };
  }, []);

  // Cleared tiles shrink into the center of where they were.
  const rects = useMemo(() => {
    const live = ITEMS.filter((i) => !cleared.has(i.id));
    const laid = squarify(
      live.map((i) => i.gb),
      { x: 0, y: 0, w: size.w, h: size.h },
    );
    const map: Record<string, Rect> = {};
    live.forEach((i, k) => (map[i.id] = laid[k]));
    for (const i of ITEMS) {
      if (map[i.id]) continue;
      map[i.id] = collapsed[i.id] ?? { x: size.w / 2, y: size.h / 2, w: 0, h: 0 };
    }
    return map;
  }, [cleared, collapsed, size]);

  const freed = ITEMS.filter((i) => cleared.has(i.id)).reduce((a, i) => a + i.gb, 0);
  const free = START_FREE + freed;
  const allDone = freed >= SAFE_TOTAL - 0.01;

  const onTile = (item: Item) => {
    if (item.verdict === "safe") {
      const r = rects[item.id];
      setCollapsed((c) => ({ ...c, [item.id]: { x: r.x + r.w / 2, y: r.y + r.h / 2, w: 0, h: 0 } }));
      setCleared((c) => new Set(c).add(item.id));
      setStatus({ text: `Cleared ${item.name}: ${item.gb} GB back. It is recreated when you need it.`, tone: "safe" });
      return;
    }
    setStatus({ text: item.note, tone: item.verdict });
    setShake(item.id);
    setTimeout(() => setShake((s) => (s === item.id ? null : s)), 450);
  };

  const reset = () => {
    setCleared(new Set());
    setCollapsed({});
    setStatus(null);
  };

  return (
    <div className="relative">
      <div className="overflow-hidden rounded-[22px] border border-line bg-card shadow-[0_40px_90px_-40px_rgba(43,37,34,0.5)]">
        <div className="flex items-center gap-4 border-b border-line px-5 py-4">
          <div className="min-w-0 flex-1">
            <div className="flex items-baseline justify-between gap-3 text-sm">
              <span className="font-semibold">Windows (C:)</span>
              <span className="font-mono text-xs text-muted tabular-nums">{free.toFixed(1)} GB free</span>
            </div>
            <div className="mt-2 h-2 overflow-hidden rounded-full bg-sand-3">
              <div
                className="h-full rounded-full bg-ember transition-[width] duration-700 ease-out"
                style={{ width: `${((DRIVE_GB - free) / DRIVE_GB) * 100}%` }}
              />
            </div>
          </div>
          <div className="flex-none text-right">
            <div className="font-mono text-[10px] tracking-[0.14em] text-muted uppercase">Freed</div>
            <div className="flex items-baseline justify-end gap-1 font-display font-extrabold text-ink">
              <Counter value={Number(freed.toFixed(1))} decimals={1} fontSize={28} fontWeight={800} textColor="#2b2522" />
              <span className="text-sm font-semibold text-muted">GB</span>
            </div>
          </div>
        </div>

        <ClickSpark sparkColor={["#f26b2a", "#7fc65a", "#f2b33d"]} sparkCount={10} sparkRadius={26} className="!h-auto">
          <div ref={box} className="relative m-2 aspect-[4/5] sm:aspect-[4/3]">
            {ITEMS.map((item, k) => {
              const r = rects[item.id];
              const gone = cleared.has(item.id);
              const roomy = r.w > 92 && r.h > 54;
              const named = r.w > 84 && r.h > 36;
              const sized = r.w > 46 && r.h > 30;
              const v = VERDICT[item.verdict];
              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => onTile(item)}
                  disabled={gone}
                  aria-label={`${item.name}, ${item.gb} GB, ${v.label}`}
                  className="group absolute p-[2px] text-left outline-none transition-[left,top,width,height,opacity] duration-700 ease-[cubic-bezier(.22,1,.36,1)]"
                  style={{
                    left: r.x,
                    top: r.y,
                    width: r.w,
                    height: r.h,
                    opacity: gone || !ready ? 0 : 1,
                    transitionDelay: settled ? "0ms" : `${k * 45}ms`,
                  }}
                >
                  <span
                    className={`relative flex h-full w-full flex-col overflow-hidden rounded-[9px] p-2.5 transition-[filter,transform] duration-200 group-hover:brightness-[1.04] group-focus-visible:ring-2 group-focus-visible:ring-ink/40 group-active:scale-[0.985] ${shake === item.id ? "animate-[nudge_0.42s_ease-in-out]" : ""}`}
                    style={{ background: TILES[k % TILES.length], cursor: item.verdict === "safe" ? "pointer" : "help" }}
                  >
                    <span className={`absolute top-2.5 right-2.5 h-2 w-2 rounded-full ${v.dot}`} />
                    {named && <span className="truncate pr-4 text-[13px] leading-4 font-semibold">{item.name}</span>}
                    {sized && <span className="mt-0.5 font-mono text-[11px] text-ink/60">{item.gb} GB</span>}
                    {roomy && (
                      <span
                        className={`mt-auto self-start rounded-full px-2 py-0.5 text-[10.5px] font-semibold opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100 ${v.chip}`}
                      >
                        {item.verdict === "safe" ? "Click to clear" : v.label}
                      </span>
                    )}
                  </span>
                </button>
              );
            })}
            {allDone && (
              <div className="absolute inset-0 grid place-items-center">
                <button
                  type="button"
                  onClick={reset}
                  className="rounded-full border border-line bg-card px-4 py-2 text-sm font-medium shadow-sm hover:bg-sand-2"
                >
                  Fill it up again
                </button>
              </div>
            )}
          </div>
        </ClickSpark>

        <div className="flex min-h-[64px] items-center gap-3 border-t border-line px-5 py-3 text-sm">
          {status ? (
            <p key={status.text} className="animate-[rise_0.35s_ease-out] leading-snug">
              <span className={`mr-2 rounded-full px-2 py-0.5 text-[11px] font-semibold ${VERDICT[status.tone].chip}`}>
                {VERDICT[status.tone].label}
              </span>
              <span className="text-muted">{status.text}</span>
            </p>
          ) : (
            <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted">
              {(Object.keys(VERDICT) as Verdict[]).map((k) => (
                <span key={k} className="flex items-center gap-1.5">
                  <span className={`h-2 w-2 rounded-full ${VERDICT[k].dot}`} />
                  {VERDICT[k].label}
                </span>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
