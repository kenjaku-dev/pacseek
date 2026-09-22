import type { Metadata } from "next";
import { JetBrains_Mono } from "next/font/google";
import { AuroraBackground } from "@/components/AuroraBackground";
import { MotionProvider } from "@/components/MotionProvider";
import "./globals.css";

const jetbrains = JetBrains_Mono({
  variable: "--font-jetbrains",
  subsets: ["latin"],
  display: "swap",
});

export const metadata: Metadata = {
  title: "pacseek — Fast AUR + repo search for Arch",
  description:
    "One Rust binary for pacman and AUR search, with a terminal UI to install and remove packages. Built with ratatui and libalpm.",
  keywords: [
    "pacseek",
    "arch linux",
    "aur",
    "pacman",
    "tui",
    "rust",
    "package search",
  ],
  openGraph: {
    title: "pacseek — Fast AUR + repo search for Arch",
    description:
      "Search official repos and the AUR, install from a TUI — one binary for Arch/Artix.",
    type: "website",
  },
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
