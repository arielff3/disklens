import Hero from "@/components/site/Hero";
import { Features, FinalCta, Footer, Safety, Statement, TourSection } from "@/components/site/Sections";

export default function Home() {
  return (
    <main className="overflow-x-clip">
      <Hero />
      <Statement />
      <Features />
      <TourSection />
      <Safety />
      <FinalCta />
      <Footer />
    </main>
  );
}
