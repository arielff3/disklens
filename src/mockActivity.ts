// Fake Activity data for the browser preview: numbers drift every second like a real machine.
import type { ActivityApp, ActivityProcess, ActivitySnapshot, EndResult, SystemPoint } from "./types";

const MB = 1024 ** 2;
const GB = 1024 ** 3;
const CORES = 16;
const RAM_TOTAL = 32 * GB;
const GPU_MEM_TOTAL = 8 * GB;

interface Spec {
  key: string;
  name: string;
  exe: string | null;
  procs: [name: string, cpu: number, ram: number, gpu: number][];
}

const SPECS: Spec[] = [
  {
    key: "c:\\program files\\google\\chrome\\application\\chrome.exe",
    name: "Google Chrome",
    exe: "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
    procs: [
      ["chrome.exe", 4.2, 610 * MB, 3.1],
      ["chrome.exe", 2.1, 420 * MB, 0],
      ["chrome.exe", 1.3, 380 * MB, 0],
      ["chrome.exe", 0.6, 290 * MB, 0],
      ["chrome.exe", 0.2, 180 * MB, 0],
    ],
  },
  {
    key: "c:\\users\\you\\appdata\\local\\programs\\microsoft vs code\\code.exe",
    name: "Visual Studio Code",
    exe: "C:\\Users\\you\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe",
    procs: [
      ["Code.exe", 3.4, 520 * MB, 0.4],
      ["Code.exe", 1.8, 690 * MB, 0],
      ["Code.exe", 0.9, 310 * MB, 0],
    ],
  },
  {
    key: "c:\\program files\\docker\\docker\\resources\\com.docker.backend.exe",
    name: "Docker Desktop Backend",
    exe: "C:\\Program Files\\Docker\\Docker\\resources\\com.docker.backend.exe",
    procs: [["com.docker.backend.exe", 1.6, 410 * MB, 0]],
  },
  { key: "name:vmmemwsl", name: "vmmemWSL", exe: null, procs: [["vmmemWSL", 2.7, 2.6 * GB, 0]] },
  {
    key: "c:\\program files\\steam\\steam.exe",
    name: "Steam",
    exe: "C:\\Program Files (x86)\\Steam\\steam.exe",
    procs: [
      ["steam.exe", 0.5, 240 * MB, 0.2],
      ["steamwebhelper.exe", 0.3, 330 * MB, 1.4],
    ],
  },
  {
    key: "c:\\users\\you\\appdata\\local\\discord\\app-1.0\\discord.exe",
    name: "Discord",
    exe: "C:\\Users\\you\\AppData\\Local\\Discord\\app-1.0\\Discord.exe",
    procs: [
      ["Discord.exe", 0.8, 280 * MB, 0.6],
      ["Discord.exe", 0.2, 140 * MB, 0],
    ],
  },
  {
    key: "windows",
    name: "",
    exe: null,
    procs: [
      ["Memory Compression", 0.1, 1.9 * GB, 0],
      ["dwm.exe", 1.1, 160 * MB, 6.5],
      ["explorer.exe", 0.6, 210 * MB, 0.3],
      ["svchost.exe", 0.4, 90 * MB, 0],
      ["svchost.exe", 0.2, 60 * MB, 0],
      ["csrss.exe", 0.1, 8 * MB, 0],
      ["System", 0.9, 1 * MB, 0],
    ],
  },
];

const ended = new Set<number>();

/** A gentle wobble per process, the same for the same second. */
const wobble = (seed: number, t: number) => 0.6 + 0.4 * Math.sin(t / 2.3 + seed * 1.7) + 0.25 * Math.sin(t / 0.9 + seed);

function apps(t: number): ActivityApp[] {
  let pid = 1000;
  return SPECS.map((spec, si) => {
    const processes: ActivityProcess[] = spec.procs
      .map(([name, cpu, ram, gpu], pi) => {
        pid += 4;
        const w = wobble(si * 7 + pi, t);
        return {
          pid,
          parent: null,
          name,
          exe: spec.key === "windows" ? null : spec.exe,
          cpu: cpu * w,
          ram: ram * (0.97 + 0.03 * w),
          gpu: gpu * w,
          gpuMem: gpu > 0 ? 180 * MB * w : 0,
        };
      })
      .filter((p) => !ended.has(p.pid));
    const sum = (f: (p: ActivityProcess) => number) => processes.reduce((a, p) => a + f(p), 0);
    return {
      key: spec.key,
      name: spec.name,
      exe: spec.exe,
      processes,
      cpu: sum((p) => p.cpu),
      ram: sum((p) => p.ram),
      gpu: sum((p) => p.gpu ?? 0),
      gpuMem: sum((p) => p.gpuMem ?? 0),
    };
  }).filter((a) => a.processes.length > 0);
}

function point(t: number): SystemPoint {
  const list = apps(t);
  const cpu = Math.min(100, 6 + list.reduce((a, x) => a + x.cpu, 0) + 8 * Math.max(0, Math.sin(t / 7)));
  const ramUsed = 9.5 * GB + list.reduce((a, x) => a + x.ram, 0);
  const gpu = Math.min(100, list.reduce((a, x) => a + (x.gpu ?? 0), 0));
  return { cpu, ramUsed, gpu };
}

export function mockActivitySnapshot(): ActivitySnapshot {
  const t = Math.floor(Date.now() / 1000);
  const history = Array.from({ length: 60 }, (_, i) => point(t - 59 + i));
  const now = history[history.length - 1];
  const list = apps(t).sort((a, b) => b.cpu - a.cpu);
  return {
    takenAt: t * 1000,
    system: {
      cpu: now.cpu,
      cores: CORES,
      ramUsed: now.ramUsed,
      ramTotal: RAM_TOTAL,
      gpu: now.gpu,
      gpuMemUsed: list.reduce((a, x) => a + (x.gpuMem ?? 0), 0) + 1.1 * GB,
      gpuMemTotal: GPU_MEM_TOTAL,
    },
    history,
    apps: list,
  };
}

export function mockActivityEnd(pids: number[]): EndResult[] {
  const names = new Map(apps(Math.floor(Date.now() / 1000)).flatMap((a) => a.processes.map((p) => [p.pid, p.name] as const)));
  return pids.map((pid) => {
    const name = names.get(pid);
    if (!name) return { pid, outcome: "gone" };
    if (["csrss.exe", "svchost.exe", "dwm.exe", "System"].includes(name)) return { pid, outcome: "refused" };
    if (name === "vmmemWSL") return { pid, outcome: "denied" };
    ended.add(pid);
    return { pid, outcome: "ended" };
  });
}
