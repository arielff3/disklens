import type { RefObject } from "react";
import { CloseIcon, SearchIcon } from "../icons";
import { S } from "../strings";

interface Props {
  value: string;
  disabled: boolean;
  inputRef: RefObject<HTMLInputElement | null>;
  onChange: (value: string) => void;
}

export function SearchBox({ value, disabled, inputRef, onChange }: Props) {
  return (
    <div className={`search${disabled ? " disabled" : ""}`} title={S.searchHint}>
      <SearchIcon />
      <input
        ref={inputRef}
        value={value}
        disabled={disabled}
        placeholder={S.searchPlaceholder}
        spellCheck={false}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            onChange("");
            e.currentTarget.blur();
          }
        }}
      />
      {value && (
        <button className="search-clear" onClick={() => onChange("")} title={S.clearSearch} aria-label={S.clearSearch}>
          <CloseIcon />
        </button>
      )}
    </div>
  );
}
