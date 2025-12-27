export interface SelectListItem<T extends string = string> {
  id: T;
  label: string;
  description?: string;
}

interface SelectListProps<T extends string = string> {
  items: SelectListItem<T>[];
  value: T | null;
  onChange: (value: T) => void;
  disabled?: boolean;
}

function CheckIcon() {
  return (
    <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2.5}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M4.5 12.75l6 6 9-13.5" />
    </svg>
  );
}

export function SelectList<T extends string = string>({
  items,
  value,
  onChange,
  disabled = false,
}: SelectListProps<T>) {
  if (items.length === 0) {
    return (
      <p className="text-text-muted text-xs py-4 text-center">
        No options available
      </p>
    );
  }

  return (
    <ul className="border border-dark-border rounded overflow-hidden divide-y divide-dark-border">
      {items.map((item) => {
        const isSelected = value === item.id;

        return (
          <li key={item.id}>
            <button
              type="button"
              onClick={() => onChange(item.id)}
              disabled={disabled}
              className={`
                w-full flex items-center gap-3 px-3 py-2.5 text-left transition-colors
                ${isSelected ? 'bg-accent-blue/10' : 'hover:bg-dark-borderSubtle'}
                ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
              `}
            >
              <span
                className={`
                  w-5 h-5 flex items-center justify-center flex-shrink-0
                  ${isSelected ? 'text-accent-blue' : 'text-transparent'}
                `}
              >
                <CheckIcon />
              </span>
              <div className="flex-1 min-w-0">
                <span className={`text-xs font-medium block ${isSelected ? 'text-accent-blue' : 'text-text-primary'}`}>
                  {item.label}
                </span>
                {item.description && (
                  <span className="text-text-muted text-xs mt-0.5 block font-mono truncate">
                    {item.description}
                  </span>
                )}
              </div>
            </button>
          </li>
        );
      })}
    </ul>
  );
}
