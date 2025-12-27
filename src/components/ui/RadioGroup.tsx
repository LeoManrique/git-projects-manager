export interface RadioOption<T extends string = string> {
  id: T;
  label: string;
  description?: string;
}

interface RadioGroupProps<T extends string = string> {
  options: RadioOption<T>[];
  value: T | null;
  onChange: (value: T) => void;
  name: string;
  disabled?: boolean;
}

export function RadioGroup<T extends string = string>({
  options,
  value,
  onChange,
  name,
  disabled = false,
}: RadioGroupProps<T>) {
  if (options.length === 0) {
    return (
      <p className="text-text-muted text-xs py-4 text-center">
        No options available
      </p>
    );
  }

  return (
    <div className="space-y-1">
      {options.map((option) => {
        const isSelected = value === option.id;

        return (
          <label
            key={option.id}
            className={`
              flex items-start gap-3 p-2.5 rounded border cursor-pointer transition-all
              ${isSelected
                ? 'bg-accent-blue/10 border-accent-blue/30'
                : 'bg-dark-surface border-dark-border hover:border-dark-borderStrong'
              }
              ${disabled ? 'opacity-50 cursor-not-allowed' : ''}
            `}
          >
            <input
              type="radio"
              name={name}
              value={option.id}
              checked={isSelected}
              onChange={() => onChange(option.id)}
              disabled={disabled}
              className="sr-only"
            />
            <span
              className={`
                mt-0.5 w-4 h-4 rounded-full border-2 flex items-center justify-center flex-shrink-0 transition-colors
                ${isSelected
                  ? 'border-accent-blue bg-accent-blue'
                  : 'border-dark-borderStrong bg-dark-bg'
                }
              `}
            >
              {isSelected && (
                <span className="w-1.5 h-1.5 rounded-full bg-white" />
              )}
            </span>
            <div className="flex-1 min-w-0">
              <span className="text-text-primary text-xs font-medium block">
                {option.label}
              </span>
              {option.description && (
                <span className="text-text-muted text-xs mt-0.5 block font-mono truncate">
                  {option.description}
                </span>
              )}
            </div>
          </label>
        );
      })}
    </div>
  );
}
