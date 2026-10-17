// BrandMark — pakmap's logogram: a chunky lowercase "p" whose bowl carries a
// map-pin notch (the mapping app) with a sky "package" dot. Deliberately
// geometric + thick: it must read at a 32×32 favicon AND as a window icon.
// The inline SVG (UI) and the generated PNG icon set
// (gui/src-tauri/icons/*, rendered from pakmap-logo.svg) share the geometry.
export function BrandMark({ size = 20 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 64 64"
      aria-hidden="true"
      focusable="false"
    >
      {/* tile */}
      <rect x="1" y="1" width="62" height="62" rx="14" fill="#080d17" />
      {/* the "p" — stem */}
      <rect x="17" y="14" width="9" height="38" rx="4" fill="#22c55e" />
      {/* the "p" — bowl */}
      <path
        d="M26 22 h10 c7.2 0 13 5.8 13 13 s-5.8 13 -13 13 h-10 v-9 h10 c2.2 0 4 -1.8 4 -4 s-1.8 -4 -4 -4 h-10 Z"
        fill="#22c55e"
      />
      {/* package pin dot */}
      <circle cx="42.5" cy="14.5" r="5.5" fill="#38bdf8" />
    </svg>
  );
}
