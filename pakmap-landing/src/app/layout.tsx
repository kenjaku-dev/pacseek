import type { Metadata, Viewport } from "next";
import { JetBrains_Mono } from "next/font/google";
import { AuroraBackground } from "@/components/AuroraBackground";
import { MotionProvider } from "@/components/MotionProvider";
import "./globals.css";

const SITE_URL = "https://kenjaku-dev.github.io/pakmap";
const OG_IMAGE = "https://opengraph.githubassets.com/1/kenjaku-dev/pakmap";

const jetbrains = JetBrains_Mono({
  variable: "--font-jetbrains",
  subsets: ["latin"],
  display: "swap",
});

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: "pakmap — Fast AUR + repo search for Arch",
  description:
    "One Rust binary for pacman and AUR search, with a terminal UI to install and remove packages. Built with ratatui and libalpm.",
  keywords: [
    "pakmap",
    "arch linux",
    "aur",
    "pacman",
    "flatpak",
    "flathub",
    "tui",
    "rust",
    "package search",
  ],
  alternates: {
    canonical: "./",
  },
  openGraph: {
    title: "pakmap — Fast AUR + repo search for Arch",
    description:
      "Search official repos, the AUR, and Flatpak; install from a TUI — one binary for Arch/Artix.",
    type: "website",
    images: [
      {
        url: OG_IMAGE,
        width: 1200,
        height: 600,
        alt: "pakmap on GitHub",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "pakmap — Fast AUR + repo search for Arch",
    description:
      "Search official repos, the AUR, and Flatpak from a TUI — one binary for Arch/Artix.",
    images: [OG_IMAGE],
  },
};

export const viewport: Viewport = {
  themeColor: "#080d17",
  colorScheme: "dark",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en" className={`${jetbrains.variable} h-full antialiased`}>
      <body className="min-h-full flex flex-col">
        <AuroraBackground />
        <MotionProvider>{children}</MotionProvider>
      </body>
    </html>
  );
}
