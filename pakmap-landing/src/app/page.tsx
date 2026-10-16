import { Features } from "@/components/Features";
import { Footer } from "@/components/Footer";
import { Hero } from "@/components/Hero";
import { HowItWorks } from "@/components/HowItWorks";
import { Install } from "@/components/Install";
import { SiteHeader } from "@/components/chrome/SiteHeader";

export default function Home() {
  return (
    <>
      <SiteHeader />
      <main id="main" tabIndex={-1} className="flex-1 outline-none">
        <Hero />
        <Features />
        <HowItWorks />
        <Install />
      </main>
      <Footer />
    </>
  );
}
