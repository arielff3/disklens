import type { Metadata } from "next";
import { Bricolage_Grotesque, DM_Mono, Instrument_Sans } from "next/font/google";
import { SITE_URL } from "@/lib/site";
import "./globals.css";

const bricolage = Bricolage_Grotesque({ variable: "--font-bricolage", subsets: ["latin"] });
const instrument = Instrument_Sans({ variable: "--font-instrument", subsets: ["latin"] });
const dmMono = DM_Mono({ variable: "--font-dm-mono", subsets: ["latin"], weight: ["400", "500"] });

const description =
  "See where your Windows disk space went, and clean it up safely. Free and open source: no account, no telemetry, nothing leaves your PC.";

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: "Disklens: see where your Windows disk space went",
  description,
  openGraph: { title: "Disklens", description, type: "website" },
  twitter: { card: "summary_large_image", title: "Disklens", description },
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html
      lang="en"
      className={`${bricolage.variable} ${instrument.variable} ${dmMono.variable} h-full antialiased`}
    >
      <body className="min-h-full font-sans">{children}</body>
    </html>
  );
}
