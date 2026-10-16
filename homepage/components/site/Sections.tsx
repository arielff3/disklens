import type { ComponentType, SVGProps } from "react";
import AnimatedContent from "@/components/reactbits/AnimatedContent";
import ScrollReveal from "@/components/reactbits/ScrollReveal";
import SpotlightCard from "@/components/reactbits/SpotlightCard";
import StarBorder from "@/components/reactbits/StarBorder";
import { DOWNLOAD, RELEASES, REPO } from "@/lib/site";
import Tour from "./Tour";
import { AppsIcon, ChartIcon, CodeIcon, ExploreIcon, Logo, ShieldIcon, SparkIcon, WindowsIcon } from "./icons";

function Kicker({ children }: { children: string }) {
  return (
    <p className="flex items-center gap-3 font-mono text-xs tracking-[0.18em] text-ember uppercase">
      <span className="h-[3px] w-7 rounded-full bg-ember" />
      {children}
    </p>
  );
}

export function Statement() {
  return (
    <section className="mx-auto max-w-4xl px-6 py-20 sm:py-28">
      <ScrollReveal
        effect="rise"
        baseOpacity={0.12}
        enableBlur
        blurStrength={4}
        textClassName="font-display text-[clamp(1.8rem,4vw,3.1rem)] leading-[1.15] font-bold tracking-[-0.025em]"
      >
        Windows shows you a gray bar called System and leaves you guessing. Disklens names every folder, says whether
        it is safe to clear, and clears it for you.
      </ScrollReveal>
    </section>
  );
}

type Feature = {
  title: string;
  body: string;
  examples: string;
  tile: string;
  Icon: ComponentType<SVGProps<SVGSVGElement>>;
};

const FEATURES: Feature[] = [
  {
    title: "Explore",
    body: "Every folder on a treemap, sized by what it really takes on disk. Scans are saved and stay current while you work.",
    examples: "treemap · file types · instant reopen",
    tile: "bg-tile-0",
    Icon: ExploreIcon,
  },
  {
    title: "System Data",
    body: "The hidden space, named: each item says whether it is safe to clear, worth a look first, or managed by Windows.",
    examples: "WinSxS · hiberfil.sys · WSL disks",
    tile: "bg-tile-1",
    Icon: WindowsIcon,
  },
  {
    title: "Projects",
    body: "Build folders from projects you left behind, with how long each one has been untouched.",
    examples: "node_modules · target · .venv · .next",
    tile: "bg-tile-2",
    Icon: CodeIcon,
  },
  {
    title: "Changes",
    body: "What grew or shrank since an earlier scan, down to the folders that explain it.",
    examples: "Docker +9 GB · Downloads +2.1 GB",
    tile: "bg-tile-3",
    Icon: ChartIcon,
  },
  {
    title: "Clean up",
    body: "Safe-to-clear caches and temp folders in one checklist. Apps that are open are left alone.",
    examples: "temp · update downloads · caches",
    tile: "bg-tile-5",
    Icon: SparkIcon,
  },
  {
    title: "Apps",
    body: "Installed programs and Store apps by their real footprint, data folders included. Uninstall, then remove what they left.",
    examples: "programs · Store apps · leftovers",
    tile: "bg-tile-4",
    Icon: AppsIcon,
  },
];

export function Features() {
  return (
    <section id="features" className="mx-auto max-w-6xl scroll-mt-10 px-6 py-16">
      <AnimatedContent distance={40} className="max-w-2xl">
        <Kicker>Features</Kicker>
        <h2 className="mt-4 font-display text-4xl font-extrabold tracking-[-0.03em] sm:text-5xl">
          One question per tab.
        </h2>
        <p className="mt-4 text-lg text-muted">
          Where did the space go, what is it, and can it go? Disklens answers each one in its own place.
        </p>
      </AnimatedContent>
      <AnimatedContent distance={50} stagger={0.08} className="mt-12 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {FEATURES.map(({ title, body, examples, tile, Icon }) => (
          <SpotlightCard
            key={title}
            theme="light"
            spotlightColor="rgba(242, 107, 42, 0.18)"
            className="flex h-full flex-col rounded-2xl p-6"
          >
            <span className={`grid h-11 w-11 place-items-center rounded-xl text-ink ${tile}`}>
              <Icon className="h-5 w-5" />
            </span>
            <h3 className="mt-5 font-display text-xl font-bold tracking-[-0.01em]">{title}</h3>
            <p className="mt-2 flex-1 leading-relaxed text-muted">{body}</p>
            <p className="mt-5 font-mono text-xs text-ink/70">{examples}</p>
          </SpotlightCard>
        ))}
      </AnimatedContent>
    </section>
  );
}

export function TourSection() {
  return (
    <section className="mx-auto grid max-w-6xl items-center gap-12 px-6 py-20 lg:grid-cols-[0.8fr_1.2fr]">
      <AnimatedContent distance={40}>
        <Kicker>The app</Kicker>
        <h2 className="mt-4 font-display text-4xl font-extrabold tracking-[-0.03em] sm:text-5xl">
          Built for Windows, from the drive up.
        </h2>
        <p className="mt-4 text-lg leading-relaxed text-muted">
          A Rust scanner reads your drive in parallel and saves each scan, so it opens instantly next time and keeps
          itself current while you work. Sand, Ink and Lagoon themes, in light and dark.
        </p>
        <p className="mt-6 font-mono text-xs text-muted">Drag the cards, or wait for the next one.</p>
      </AnimatedContent>
      <AnimatedContent distance={60} scale={0.96}>
        <Tour />
      </AnimatedContent>
    </section>
  );
}

const SAFETY = [
  ["Recycle Bin first", "Clean up and app leftovers go to the Recycle Bin. Windows asks before anything too big for it."],
  ["Protected folders stay put", "Drive roots, Windows, Program Files, your user folder, AppData and the folders that hold them are never touched."],
  ["Links are never followed", "Junctions and symbolic links are skipped, so shared stores like pnpm's stay intact."],
  ["Only what you were shown", "The backend refuses any path it did not offer you, and open apps are skipped."],
];

export function Safety() {
  return (
    <section id="safety" className="mx-auto max-w-6xl scroll-mt-10 px-6 py-20">
      <div className="rounded-[28px] border border-line bg-sand-2 p-8 sm:p-12">
        <AnimatedContent distance={40} className="max-w-2xl">
          <Kicker>Safety</Kicker>
          <h2 className="mt-4 font-display text-4xl font-extrabold tracking-[-0.03em] sm:text-5xl">
            It deletes things. Carefully.
          </h2>
          <p className="mt-4 text-lg text-muted">
            Everything runs on your PC. No account, no telemetry, nothing is uploaded.
          </p>
        </AnimatedContent>
        <AnimatedContent distance={40} stagger={0.08} className="mt-10 grid gap-x-10 gap-y-7 sm:grid-cols-2">
          {SAFETY.map(([title, body]) => (
            <div key={title} className="flex gap-4">
              <ShieldIcon className="mt-0.5 h-6 w-6 flex-none text-leaf" />
              <div>
                <h3 className="font-semibold">{title}</h3>
                <p className="mt-1 leading-relaxed text-muted">{body}</p>
              </div>
            </div>
          ))}
        </AnimatedContent>
      </div>
    </section>
  );
}

export function FinalCta() {
  return (
    <section className="relative mx-auto max-w-4xl px-6 py-24 text-center sm:py-32">
      <AnimatedContent distance={50}>
        <Logo className="mx-auto h-16 w-16" />
        <h2 className="mt-8 font-display text-[clamp(2.4rem,6vw,4.4rem)] leading-[1.02] font-extrabold tracking-[-0.035em]">
          Get your space back.
        </h2>
        <p className="mx-auto mt-5 max-w-xl text-lg text-muted">Free, open source, and about a minute to install.</p>
        <div className="mt-9 flex justify-center">
          <StarBorder
            as="a"
            href={DOWNLOAD}
            theme="light"
            color="#f26b2a"
            radius={999}
            backgroundColor="#2b2522"
            textColor="#f7f2eb"
            className="font-medium"
          >
            <span className="flex items-center gap-2.5 px-3 py-1">
              <WindowsIcon className="h-4 w-4" />
              Download for Windows
            </span>
          </StarBorder>
        </div>
        <p className="mx-auto mt-6 max-w-md text-sm text-muted">
          The installer isn&apos;t code-signed yet, so Windows may warn you: choose{" "}
          <span className="font-medium text-ink">More info → Run anyway</span>.
        </p>
      </AnimatedContent>
    </section>
  );
}

export function Footer() {
  return (
    <footer className="border-t border-line">
      <div className="mx-auto flex max-w-6xl flex-col gap-4 px-6 py-10 text-sm text-muted sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-center gap-2.5">
          <Logo className="h-5 w-5" />
          <span>
            Disklens · MIT license · by{" "}
            <a href="https://github.com/arielff3" className="text-ink hover:text-ember-strong">
              Ariel
            </a>
          </span>
        </div>
        <div className="flex gap-5">
          <a href={REPO} className="hover:text-ink">
            Source
          </a>
          <a href={RELEASES} className="hover:text-ink">
            Releases
          </a>
          <a href={`${REPO}/issues`} className="hover:text-ink">
            Issues
          </a>
        </div>
      </div>
      <p className="mx-auto max-w-6xl px-6 pb-10 text-xs text-muted/80">
        Inspired by Diskzy for macOS. Disklens is an independent project, not affiliated with Diskzy.
      </p>
    </footer>
  );
}

