/**
 * Textarea Component
 */

import React, { useState } from 'react';

export interface TextareaProps
  extends Omit<React.TextareaHTMLAttributes<HTMLTextAreaElement>, 'onChange'> {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  label?: string;
  helperText?: string;
  error?: string;
  rows?: number;
  maxLength?: number;
  showCount?: boolean;
  autoHeight?: boolean;
  minHeight?: string;
  maxHeight?: string;
  disabled?: boolean;
  className?: string;
}

export const Textarea: React.FC<TextareaProps> = ({
  value,
  onChange,
  placeholder,
  label,
  helperText,
  error,
  rows = 4,
  maxLength,
  showCount = false,
  autoHeight = false,
  minHeight,
  maxHeight,
  disabled = false,
  className = '',
  ...props
}) => {
  const [isFocused, setIsFocused] = useState(false);

  const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    onChange(e.target.value);
  };

  const handleInput = (e: React.FormEvent<HTMLTextAreaElement>) => {
    const target = e.target as HTMLTextAreaElement;
    if (autoHeight) {
      target.style.height = 'auto';
      target.style.height = `${target.scrollHeight}px`;
    }
  };

  const hasError = !!error;
  const charCount = value.length;
  const showMaxLength = maxLength && showCount;

  return (
    <div className={`w-full ${className}`}>
      {/* Label */}
      {label && (
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5">
          {label}
        </label>
      )}

      {/* Textarea Container */}
      <div className="relative">
        <textarea
          value={value}
          onChange={handleChange}
          onFocus={() => setIsFocused(true)}
          onBlur={() => setIsFocused(false)}
          onInput={handleInput}
          placeholder={placeholder}
          rows={rows}
          maxLength={maxLength}
          disabled={disabled}
          className={`
            w-full px-3 py-2
            text-sm text-gray-900 dark:text-white
            bg-white dark:bg-gray-800
            placeholder-gray-400 dark:placeholder-gray-500
            border rounded-lg
            transition-all duration-200
            resize-none
            focus:outline-none focus:ring-2
            disabled:opacity-50 disabled:cursor-not-allowed
            ${hasError
              ? 'border-red-500 focus:border-red-500 focus:ring-red-200 dark:focus:ring-red-900'
              : isFocused
                ? 'border-primary-500 focus:ring-primary-200 dark:focus:ring-primary-900'
                : 'border-gray-300 dark:border-gray-600 hover:border-gray-400 dark:hover:border-gray-500'
            }
          `}
          style={{
            minHeight: autoHeight ? minHeight : undefined,
            maxHeight: autoHeight ? maxHeight : undefined,
          }}
          {...props}
        />

        {/* Character Count */}
        {showMaxLength && (
          <div className="absolute bottom-2 right-2 text-xs text-gray-400 dark:text-gray-500">
            {charCount}/{maxLength}
          </div>
        )}
      </div>

      {/* Helper Text / Error */}
      {(helperText || error) && (
        <p
          className={`mt-1.5 text-sm ${
            hasError
              ? 'text-red-600 dark:text-red-400'
              : 'text-gray-500 dark:text-gray-400'
          }`}
        >
          {error || helperText}
        </p>
      )}
    </div>
  );
};

export default Textarea;
