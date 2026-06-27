/**
 * Tooltip Component
 */

import React, { useState, useRef, useCallback } from 'react';

export interface TooltipProps {
  content: React.ReactNode;
  children: React.ReactElement;
  position?: 'top' | 'bottom' | 'left' | 'right';
  delay?: number;
  disabled?: boolean;
  className?: string;
}

export const Tooltip: React.FC<TooltipProps> = ({
  content,
  children,
  position = 'top',
  delay = 200,
  disabled = false,
  className = '',
}) => {
  const [isVisible, setIsVisible] = useState(false);
  const timeoutRef = useRef<number | null>(null);

  const showTooltip = useCallback(() => {
    if (disabled) return;
    timeoutRef.current = window.setTimeout(() => {
      setIsVisible(true);
    }, delay);
  }, [delay, disabled]);

  const hideTooltip = useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    setIsVisible(false);
  }, []);

  const positionStyles = {
    top: {
      tooltip: 'bottom-full left-1/2 -translate-x-1/2 mb-2',
      arrow: 'top-full left-1/2 -translate-x-1/2 border-t-gray-900 border-x-transparent border-b-transparent',
    },
    bottom: {
      tooltip: 'top-full left-1/2 -translate-x-1/2 mt-2',
      arrow: 'bottom-full left-1/2 -translate-x-1/2 border-b-gray-900 border-x-transparent border-t-transparent',
    },
    left: {
      tooltip: 'right-full top-1/2 -translate-y-1/2 mr-2',
      arrow: 'left-full top-1/2 -translate-y-1/2 border-l-gray-900 border-y-transparent border-r-transparent',
    },
    right: {
      tooltip: 'left-full top-1/2 -translate-y-1/2 ml-2',
      arrow: 'right-full top-1/2 -translate-y-1/2 border-r-gray-900 border-y-transparent border-l-transparent',
    },
  };

  const { tooltip: tooltipPosition, arrow: arrowPosition } = positionStyles[position];

  return (
    <div className={`relative inline-flex ${className}`}>
      {React.cloneElement(children, {
        onMouseEnter: (e: React.MouseEvent) => {
          showTooltip();
          if (children.props.onMouseEnter) {
            children.props.onMouseEnter(e);
          }
        },
        onMouseLeave: (e: React.MouseEvent) => {
          hideTooltip();
          if (children.props.onMouseLeave) {
            children.props.onMouseLeave(e);
          }
        },
        onFocus: (e: React.FocusEvent) => {
          showTooltip();
          if (children.props.onFocus) {
            children.props.onFocus(e);
          }
        },
        onBlur: (e: React.FocusEvent) => {
          hideTooltip();
          if (children.props.onBlur) {
            children.props.onBlur(e);
          }
        },
      })}

      {isVisible && (
        <div
          role="tooltip"
          className={`
            absolute z-50 ${tooltipPosition}
            px-2 py-1
            text-xs text-white
            bg-gray-900 dark:bg-gray-700
            rounded shadow-lg
            whitespace-nowrap
            pointer-events-none
            animate-fade-in
          `}
        >
          {content}
          {/* Arrow */}
          <span
            className={`
              absolute w-0 h-0
              border-4 ${arrowPosition}
            `}
          />
        </div>
      )}
    </div>
  );
};

export default Tooltip;
