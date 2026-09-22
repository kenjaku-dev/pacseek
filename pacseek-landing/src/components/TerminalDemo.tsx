"use client";

import { useEffect, useMemo, useState } from "react";
import { motion, useReducedMotion } from "framer-motion";

type ResultLine = {
  kind: "selected" | "item" | "desc";
  text: string;
};

const QUERY = "firefox";

const RESULTS: ResultLine[] = [
  { kind: "selected", text: "▸ extra/firefox 141.0-1" },
  { kind: "desc", text: "  Fast, privacy-focused browser" },
  { kind: "item", text: "  aur/firefox-bin 141.0-1 (+1240 5.2)" },
  { kind: "desc", text: "  Binary Firefox package" },
  { kind: "item", text: "  extra/firefox-i18n-en-us 141.0-1" },
  { kind: "desc", text: "  English language pack" },
];

const STATUS = " Found 10 (repo 5 aur 5)  Tab:remover ?:help";

type Phase = "typing" | "searching" | "results" | "done";

function useTerminalSequence(reduce: boolean | null) {
  const [typed, setTyped] = useState(0);
  const [visible, setVisible] = useState(0);
  const [phase, setPhase] = useState<Phase>("typing");
  const [cursorOn, setCursorOn] = useState(true);

  useEffect(() => {
    if (reduce) return;

    let cancelled = false;
    const timers: ReturnType<typeof setTimeout>[] = [];

    const wait = (ms: number) =>
      new Promise<void>((resolve) => {
        timers.push(setTimeout(resolve, ms));
      });

    const run = async () => {
      await wait(600);
      if (cancelled) return;

      for (let i = 1; i <= QUERY.length; i++) {
        if (cancelled) return;
        setTyped(i);
        await wait(70 + Math.random() * 50);
      }
      if (cancelled) return;

      setPhase("searching");
      await wait(450);
      if (cancelled) return;

      setPhase("results");
      for (let i = 1; i <= RESULTS.length; i++) {
        if (cancelled) return;
        setVisible(i);
        await wait(120);
      }
      if (cancelled) return;

      setPhase("done");
    };

    void run();

    const blink = setInterval(() => {
      if (!cancelled) setCursorOn((c) => !c);
    }, 530);

    return () => {
      cancelled = true;
      timers.forEach(clearTimeout);
      clearInterval(blink);
    };
  }, [reduce]);

  if (reduce) {
    return { typed: QUERY.length, visible: RESULTS.length, phase: "done" as Phase, cursorOn: true };
  }

  return { typed, visible, phase, cursorOn };
}

export function TerminalDemo() {
  const reduce = useReducedMotion();
  const { typed, visible, phase, cursorOn } = useTerminalSequence(reduce);
  const query = useMemo(() => QUERY.slice(0, typed), [typed]);

  return (
    <div
      role="img"
      aria-label="pacseek terminal demo: searching for firefox in official repos and the AUR"
      className="w-full overflow-hidden rounded-[11px] bg-card"
    >
      <div className="flex items-center gap-2 border-b border-border/70 bg-muted/60 px-3 py-2">
        <span className="h-2.5 w-2.5 rounded-full bg-destructive/80" aria-hidden="true" />
        <span className="h-2.5 w-2.5 rounded-full bg-yellow-500/70" aria-hidden="true" />
        <span className="h-2.5 w-2.5 rounded-full bg-accent/70" aria-hidden="true" />
        <span className="ml-2 truncate text-[11px] text-muted-foreground">
          pacseek — Search / Installed
        </span>
      </div>

      <div className="space-y-2 p-3 font-mono text-[11px] leading-relaxed sm:text-xs md:p-4">
        <div className="rounded-md border border-border/80 px-2 py-1.5 text-foreground">
          <span className="text-muted-foreground">Search</span>
          <span className="mx-1 text-border">│</span>
          <span className="text-muted-foreground">Enter to search</span>
          <div className="mt-1 min-h-[1.25rem] break-all">
            <span className="text-accent-secondary">{query}</span>
            <span
              aria-hidden="true"
              className={`ml-px inline-block h-[1em] w-[0.55em] translate-y-[0.1em] bg-accent ${
                cursorOn && phase !== "searching" ? "opacity-100" : "opacity-0"
              }`}
            />
            {phase === "searching" && (
              <motion.span
                initial={{ opacity: 0 }}
                animate={{ opacity: [0.3, 1, 0.3] }}
                transition={{ duration: 0.9, repeat: Infinity }}
                className="ml-2 text-accent"
              >
                searching…
              </motion.span>
            )}
          </div>
        </div>

        <div className="rounded-md border border-border/80 px-2 py-1.5">
          <div className="text-muted-foreground">
            Results
            {phase !== "typing" && phase !== "searching" && (
              <span className="text-foreground"> (1/{visible || RESULTS.length})</span>
            )}
          </div>

          <div className="mt-1.5 min-h-[9.5rem] space-y-0.5">
            {(phase === "results" || phase === "done"
              ? RESULTS.slice(0, visible)
              : []
            ).map((line, i) => (
              <motion.div
                key={line.text}
                initial={reduce ? false : { opacity: 0, x: -6 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ duration: 0.2, ease: "easeOut" }}
                className={
                  line.kind === "selected"
                    ? "rounded-sm bg-accent/15 px-1 text-accent"
                    : line.kind === "desc"
                      ? "pl-4 text-muted-foreground"
                      : "px-1 text-foreground"
                }
              >
                {line.text}
                {line.kind === "selected" && i === 0 && phase === "done" && (
                  <span className="ml-2 text-accent/80">Enter: install</span>
                )}
              </motion.div>
            ))}

            {phase === "typing" && (
              <p className="px-1 text-muted-foreground/70">Type a query, press Enter…</p>
            )}
          </div>
        </div>

        <div className="border-t border-border/70 pt-2 text-[10px] text-muted-foreground sm:text-[11px]">
          {phase === "done" ? (
            <span>
              <span className="text-accent">{STATUS}</span>
            </span>
          ) : (
            <span>↑↓/j k  Enter:install  i:info  /:search  q:quit</span>
          )}
        </div>
      </div>
    </div>
  );
}
