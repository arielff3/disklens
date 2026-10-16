import AnimatedContent from "@/components/reactbits/AnimatedContent";
import BlurText from "@/components/reactbits/BlurText";
import Magnet from "@/components/reactbits/Magnet";
import ShinyText from "@/components/reactbits/ShinyText";
import { DOWNLOAD, REPO, VERSION } from "@/lib/site";
import DiskMap from "./DiskMap";
import { GitHubIcon, Logo, WindowsIcon } from "./icons";

function Nav() {
  return (
    <header className="relative z-10 mx-auto flex max-w-6xl items-center justify-between px-6 py-5">
      <a href="#" className="flex items-center gap-2.5 font-display text-lg font-bold">
        <Logo className="h-7 w-7" />
        Disklens
      </a>
      <nav className="flex items-center gap-1 text-sm sm:gap-2">
        <a href="#features" className="hidden rounded-full px-3 py-2 text-muted hover:text-ink sm:block">
          Features
        </a>
        <a href="#safety" className="hidden rounded-full px-3 py-2 text-muted hover:text-ink sm:block">
          Safety
        </a>
        <a
          href={REPO}
          className="flex items-center gap-2 rounded-full border border-line bg-card px-3.5 py-2 font-medium hover:bg-sand-2"
        >
          <GitHubIcon className="h-4 w-4" />
          GitHub
        </a>
      </nav>
    </header>
  );
}

function DownloadButtons() {
  return (
    <div className="flex flex-wrap items-center gap-3">
      <Magnet padding={60} magnetStrength={4} maxOffset={14}>
        <a
          href={DOWNLOAD}
          className="flex items-center gap-2.5 rounded-full bg-ink px-6 py-3.5 font-medium text-sand shadow-[0_10px_30px_-12px_rgba(43,37,34,0.6)] transition-colors hover:bg-ember-strong"
        >
          <WindowsIcon className="h-4 w-4" />
          Download for Windows
        </a>
      </Magnet>
      <a
        href={REPO}
        className="flex items-center gap-2.5 rounded-full border border-line bg-card px-6 py-3.5 font-medium hover:bg-sand-2"
      >
        <GitHubIcon className="h-4 w-4" />
        Star on GitHub
      </a>
    </div>
  );
}

export default function Hero() {
  return (
    <section className="relative overflow-hidden">
      <div
        aria-hidden="true"
        className="pointer-events-none absolute -top-40 right-[-10%] h-[720px] w-[720px] rounded-full bg-[radial-gradient(closest-side,rgb(242_107_42/0.16),transparent)]"
      />
      <Nav />
      <div className="relative mx-auto grid max-w-6xl items-center gap-12 px-6 pt-8 pb-24 sm:pt-14 lg:grid-cols-[0.92fr_1.08fr] lg:gap-14 lg:pb-32">
        <div>
          <div className="inline-flex items-center gap-2 rounded-full border border-line bg-card/80 px-4 py-1.5 font-mono text-xs tracking-wide">
            <span className="h-1.5 w-1.5 rounded-full bg-ember" />
            <ShinyText text={`Free and open source · v${VERSION}`} color="#5f5750" shineColor="#f26b2a" speed={3} />
          </div>
          <div role="heading" aria-level={1} className="mt-6">
            <BlurText
              text="Find what's eating your disk."
              animateBy="words"
              trigger="load"
              delay={90}
              className="font-display text-[clamp(2.6rem,6.2vw,4.9rem)] leading-[0.98] font-extrabold tracking-[-0.04em]"
            />
          </div>
          <p className="mt-6 max-w-xl text-lg leading-relaxed text-muted sm:text-xl">
            Disklens maps every folder on your Windows drive, tells you what each one is, and clears only what is
            safe to clear.
          </p>
          <div className="mt-8">
            <DownloadButtons />
          </div>
          <p className="mt-4 font-mono text-xs text-muted">Windows 10 and 11 · 2.4 MB · No account, no telemetry</p>
        </div>
        <AnimatedContent distance={60} scale={0.97} duration={1} delay={0.2} trigger="mount">
          <p className="mb-3 flex items-center gap-2 font-mono text-xs text-muted">
            <span className="h-[3px] w-6 rounded-full bg-ember" />
            Try it: click a tile to clear it
          </p>
          <DiskMap />
        </AnimatedContent>
      </div>
    </section>
  );
}
