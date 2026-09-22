"use client";

import { useSpotlight } from "./useSpotlight";

type SpotlightCardProps = {
  children: React.ReactNode;
  className?: string;
};

/** Generic pointer-tracking spotlight container (see `.spotlight-card`). */
export function SpotlightCard({ children, className = "" }: SpotlightCardProps) {
  const { ref, onPointerMove } = useSpotlight<HTMLDivElement>();

  return (
    <div
      ref={ref}
      onPointerMove={onPointerMove}
      className={`spotlight-card ${className}`}
    >
      {children}
    </div>
  );
}
