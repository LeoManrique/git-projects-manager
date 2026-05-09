export interface SelectOption<T extends string = string> {
  value: T;
  label: string;
}

interface SelectProps<T extends string = string> {
  options: SelectOption<T>[];
  value: T | null;
  onChange: (value: T) => void;
  placeholder?: string;
  disabled?: boolean;
}

function ChevronIcon() {
  return (
    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
    </svg>
  );
}

export function Select<T extends string = string>({
  options,
  value,
  onChange,
  placeholder = 'Select...',
  disabled = false,
}: SelectProps<T>) {
  return (
    <div className="relative">
      <select
        value={value ?? ''}
        onChange={(e) => onChange(e.target.value as T)}
        disabled={disabled}
        className={`
          w-full appearance-none px-3 py-2 pr-10
          bg-dark-surface border border-dark-border rounded
          text-text-primary text-xs
          focus:outline-none focus:border-accent-blue/50
          disabled:opacity-50 disabled:cursor-not-allowed
          cursor-pointer
        `}
      >
        <option value="" disabled>
          {placeholder}
        </option>
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
      <span className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-text-muted">
        <ChevronIcon />
      </span>
    </div>
  );
}
