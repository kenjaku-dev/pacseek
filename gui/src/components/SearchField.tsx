// SearchField — autoselect-on-focus, Enter forces immediate run (no debounce
// wait), Esc clears. Semantic HTML: implicit form submit on Enter.
import { Search, X } from "lucide-react";
import { useEffect, useRef } from "react";

interface SearchFieldProps {
  value: string;
  onInput: (v: string) => void;
  onSubmit: () => void;
  onClear: () => void;
}

export function SearchField({ value, onInput, onSubmit, onClear }: SearchFieldProps) {
  const inputRef = useRef<HTMLInputElement>(null);

  // Primary focus: the search field is the screen's focal point (ui-design:
  // ONE primary focal point per viewport).
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  return (
    <form
      className="relative flex flex-1 items-center"
      role="search"
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit();
      }}
    >
      <Search
        aria-hidden="true"
        size={16}
        className="pointer-events-none absolute left-3 text-muted-foreground"
      />
      <input
        ref={inputRef}
        value={value}
        onChange={(e) => onInput(e.target.value)}
        onFocus={(e) => e.target.select()}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            onClear();
          }
        }}
        type="text"
        aria-label="Search packages"
        placeholder="Search packages — / to focus, Enter to search now"
        spellCheck={false}
        autoComplete="off"
        className="h-10 w-full rounded-lg border border-border bg-card-muted pl-9 pr-10 text-foreground placeholder:text-muted-foreground/70 hover:border-accent-secondary/60 focus:border-accent-secondary focus-visible:outline-none"
      />
      {value !== "" && (
        <button
          type="button"
          aria-label="Clear search"
          onClick={onClear}
          className="absolute right-3 grid size-5 cursor-pointer place-items-center rounded text-muted-foreground transition-colors hover:text-foreground"
        >
          <X size={14} />
        </button>
      )}
    </form>
  );
}
