/**
 * Switch/Toggle Component
 */

import React from 'react';

export interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: React.ReactNode;
  description?: React.ReactNode;
  disabled?: boolean;
  size?: 'sm' | 'md';
  className?: string;
}

export const Switch: React.FC<SwitchProps> = ({
  checked,
  onChange,
  label,
  description,
  disabled = false,
  size = 'md',
  className = '',
}) => {
  const sizeStyles = {
    sm: {
      track: 'w-8 h-4',
      thumb: 'h-3 w-3',
      translate: 'translate-x-4',
    },
    md: {
      track: 'w-11 h-6',
      thumb: 'h-4 w-4',
      translate: 'translate-x-5',
    },
  };

  const { track, thumb, translate } = sizeStyles[size];

  return (
    <label
      className={`
        inline-flex items-start gap-3
        ${disabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'}
        ${className}
      `}
    >
      {/* Switch Track */}
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => !disabled && onChange(!checked)}
        className={`
          ${track}
          relative inline-flex shrink-0 rounded-full
          transition-colors duration-200 ease-in-out
          focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2
          ${checked
            ? 'bg-primary-600 dark:bg-primary-500'
            : 'bg-gray-200 dark:bg-gray-700'
          }
        `}
      >
        {/* Switch Thumb */}
        <span
          className={`
            ${thumb}
            pointer-events-none inline-block rounded-full bg-white shadow
            transform transition-transform duration-200 ease-in-out
            ${checked ? translate : 'translate-x-1'}
            mt-1 ml-1
          `}
        />
      </button>

      {/* Label and Description */}
      {(label || description) && (
        <div className="flex flex-col">
          {label && (
            <span className="text-sm font-medium text-gray-900 dark:text-white">
              {label}
            </span>
          )}
          {description && (
            <span className="text-sm text-gray-500 dark:text-gray-400">
              {description}
            </span>
          )}
        </div>
      )}
    </label>
  );
};

export default Switch;
