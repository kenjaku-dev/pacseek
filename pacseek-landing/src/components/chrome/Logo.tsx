import { Terminal } from "lucide-react";
import { SITE } from "@/lib/site";

type LogoProps = {
  showWordmark?: boolean;
  iconClassName?: string;
  href?: string;
};

export function Logo({
  showWordmark = true,
  iconClassName = "h-4 w-4",
  href = "#top",
}: LogoProps) {
  const mark = (
    <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md border border-border bg-card text-accent">
      <Terminal aria-hidden="true" className={iconClassName} />
    </span>
  );

  if (!showWordmark) {
    return mark;
  }

  return (
    <a
      href={href}
      className="flex items-center gap-2 text-sm font-medium text-foreground transition-colors duration-200 hover:text-accent"
    >
      {mark}
      {SITE.name}
    </a>
  );
}
