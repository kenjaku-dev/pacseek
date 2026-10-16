"use client";

import { useState } from "react";
import { Check, Copy } from "lucide-react";

type CopyButtonProps = {
  text: string;
  label?: string;
  className?: string;
};

export function CopyButton({ text, label = "Copy command", className = "" }: CopyButtonProps) {
  const [copied, setCopied] = useState(false);
  const [failed, setFailed] = useState(false);

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setFailed(false);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      setFailed(true);
      setTimeout(() => setFailed(false), 2000);
    }
  }

  return (
    <button
      type="button"
      onClick={handleCopy}
      aria-label={copied ? "Copied" : failed ? "Copy failed" : label}
      className={`inline-flex cursor-pointer items-center gap-1.5 rounded-md border border-border bg-muted px-2.5 py-1.5 text-xs text-muted-foreground transition-colors duration-200 hover:border-accent/50 hover:text-foreground ${className}`}
    >
      {copied ? (
        <Check aria-hidden="true" className="h-3.5 w-3.5 text-accent" />
      ) : (
        <Copy aria-hidden="true" className="h-3.5 w-3.5" />
      )}
      <span>{copied ? "Copied" : "Copy"}</span>
      {/* Announce the state change — aria-label swaps are not announced by
          screen readers, a live region is */}
      <span role="status" className="sr-only">
        {copied ? "Copied to clipboard" : failed ? "Copy failed — clipboard unavailable" : ""}
      </span>
    </button>
  );
}
