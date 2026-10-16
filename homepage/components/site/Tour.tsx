"use client";

import CardSwap from "@/components/reactbits/CardSwap";

const SHOTS = [
  { title: "Explore", image: "/screenshots/explore.png", alt: "Explore: folders as a list and a treemap" },
  { title: "System Data", image: "/screenshots/system-data.png", alt: "System Data: what Windows keeps, with verdicts" },
  { title: "Projects", image: "/screenshots/projects.png", alt: "Projects: build folders by project" },
  { title: "Changes", image: "/screenshots/changes.png", alt: "Changes: what grew since the last scan" },
  { title: "Apps", image: "/screenshots/apps.png", alt: "Apps: programs by footprint" },
];

export default function Tour() {
  return (
    <div className="aspect-[4/3] w-full">
      <CardSwap
        items={SHOTS}
        theme="light"
        width={640}
        height={430}
        cardDistance={50}
        verticalDistance={56}
        delay={3500}
        cardRadius={16}
      />
    </div>
  );
}
