import Image from "next/image";
import AnimatedContent from "@/components/reactbits/AnimatedContent";
import DotGrid from "@/components/reactbits/DotGrid";
import Magnet from "@/components/reactbits/Magnet";
import RotatingText from "@/components/reactbits/RotatingText";
import ShinyText from "@/components/reactbits/ShinyText";
import { DOWNLOAD, REPO, VERSION } from "@/lib/site";
import { GitHubIcon, Logo, WindowsIcon } from "./icons";

const CULPRITS = ["node_modules", "WinSxS", "Docker disks", "hiberfil.sys", "browser caches", "old downloads"];

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

export function DownloadButtons({ center = true }: { center?: boolean }) {
  return (
    <div className={`flex flex-wrap items-center gap-3 ${center ? "justify-center" : ""}`}>
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
      <div className="absolute inset-0 [mask-image:radial-gradient(ellipse_70%_60%_at_50%_35%,black,transparent)]">
        <DotGrid dotSize={4} gap={24} baseColor="#ddd0bf" activeColor="#f26b2a" proximity={130} glow={0.2} />
      </div>
      <Nav />
      <div className="pointer-events-none relative mx-auto max-w-5xl px-6 pt-14 pb-16 text-center sm:pt-20">
        <div className="pointer-events-auto inline-flex items-center gap-2 rounded-full border border-line bg-card/80 px-4 py-1.5 font-mono text-xs tracking-wide backdrop-blur">
          <span className="h-1.5 w-1.5 rounded-full bg-ember" />
          <ShinyText text={`Free and open source · v${VERSION}`} color="#5f5750" shineColor="#f26b2a" speed={3} />
        </div>
        <h1 className="mt-7 font-display text-[clamp(2.25rem,7.5vw,5.6rem)] leading-[1.02] font-extrabold tracking-[-0.035em]">
          Your disk is full of
          <br />
          <RotatingText
            texts={CULPRITS}
            highlight
            highlightColor="#f26b2a"
            rotationInterval={2200}
            staggerDuration={0.02}
            mainClassName="mt-2 inline-flex overflow-hidden rounded-[0.3em] px-[0.3em] pb-[0.06em] align-bottom"
          />
        </h1>
        <p className="mx-auto mt-7 max-w-2xl text-lg leading-relaxed text-muted sm:text-xl">
          Disklens shows where your Windows disk space went, explains what each folder is, and clears what is safe
          to clear.
        </p>
        <div className="pointer-events-auto mt-9">
          <DownloadButtons />
        </div>
        <p className="mt-4 font-mono text-xs text-muted">Windows 10 and 11 · 2.4 MB · No account, no telemetry</p>
      </div>
      <AnimatedContent distance={80} scale={0.96} duration={1.1} delay={0.15} className="relative px-4 pb-24 sm:px-6">
        <div className="mx-auto max-w-5xl overflow-hidden rounded-2xl border border-line bg-card shadow-[0_40px_100px_-40px_rgba(43,37,34,0.45)]">
          <Image
            src="/demo.gif"
            alt="A tour of Disklens: the treemap, System Data, Projects and Changes"
            width={1920}
            height={1080}
            unoptimized
            priority
            className="block h-auto w-full"
          />
        </div>
      </AnimatedContent>
    </section>
  );
}
