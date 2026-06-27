/**
 * Tabs Component
 */

import React, { useId } from 'react';

export interface Tab {
  id: string;
  label: React.ReactNode;
  icon?: React.ReactNode;
  disabled?: boolean;
  content?: React.ReactNode;
}

export interface TabsProps {
  tabs: Tab[];
  activeTab: string;
  onChange: (tabId: string) => void;
  variant?: 'underline' | 'pill' | 'bordered';
  size?: 'sm' | 'md';
  fullWidth?: boolean;
  renderTab?: (tab: Tab) => React.ReactNode;
}

export const Tabs: React.FC<TabsProps> = ({
  tabs,
  activeTab,
  onChange,
  variant = 'underline',
  size = 'md',
  fullWidth = false,
}) => {
  const instanceId = useId();

  const sizeStyles = {
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2 text-sm',
  };

  const getTabClassName = (tab: Tab, isActive: boolean) => {
    const baseClass = `
      ${sizeStyles[size]}
      ${fullWidth ? 'flex-1' : ''}
      inline-flex items-center justify-center gap-2
      font-medium transition-all duration-200
      focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2
      disabled:opacity-50 disabled:cursor-not-allowed
      ${variant === 'underline' ? 'relative' : ''}
    `;

    if (tab.disabled) {
      return `${baseClass} cursor-not-allowed text-gray-400 dark:text-gray-500`;
    }

    switch (variant) {
      case 'underline':
        return `
          ${baseClass}
          text-gray-600 dark:text-gray-400
          hover:text-gray-900 dark:hover:text-white
          ${isActive
            ? 'text-primary-600 dark:text-primary-400'
            : ''
          }
        `;
      case 'pill':
        return `
          ${baseClass}
          rounded-lg
          ${isActive
            ? 'bg-primary-600 text-white shadow-sm'
            : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800'
          }
        `;
      case 'bordered':
        return `
          ${baseClass}
          border border-gray-300 dark:border-gray-600 rounded-lg
          ${isActive
            ? 'bg-white dark:bg-gray-800 text-gray-900 dark:text-white border-primary-500'
            : 'bg-gray-50 dark:bg-gray-900 text-gray-600 dark:text-gray-400 hover:bg-white dark:hover:bg-gray-800'
          }
        `;
      default:
        return baseClass;
    }
  };

  return (
    <div className="w-full">
      {/* Tab List */}
      <div
        role="tablist"
        className={`
          ${variant === 'underline' ? 'relative flex border-b border-gray-200 dark:border-gray-700' : 'flex gap-2'}
          ${fullWidth ? '' : 'inline-flex'}
        `}
      >
        {tabs.map((tab) => (
          <button
            key={tab.id}
            role="tab"
            id={`tab-${instanceId}-${tab.id}`}
            aria-selected={activeTab === tab.id}
            aria-controls={`tabpanel-${instanceId}-${tab.id}`}
            disabled={tab.disabled}
            tabIndex={activeTab === tab.id ? 0 : -1}
            onClick={() => !tab.disabled && onChange(tab.id)}
            className={getTabClassName(tab, activeTab === tab.id)}
          >
            {tab.icon && <span className="flex-shrink-0">{tab.icon}</span>}
            {typeof tab.label === 'string' ? (
              <span>{tab.label}</span>
            ) : (
              tab.label
            )}
            {/* Underline indicator */}
            {variant === 'underline' && activeTab === tab.id && (
              <span
                className={`
                  absolute bottom-0 left-0 h-0.5 w-full
                  bg-primary-600 dark:bg-primary-400
                `}
              />
            )}
          </button>
        ))}
      </div>

      {/* Tab Panels */}
      {tabs.map((tab) => {
        if (tab.id !== activeTab) return null;
        return (
          <div
            key={tab.id}
            role="tabpanel"
            id={`tabpanel-${instanceId}-${tab.id}`}
            aria-labelledby={`tab-${instanceId}-${tab.id}`}
            tabIndex={0}
            className="mt-4 animate-fade-in"
          >
            {tab.content}
          </div>
        );
      })}
    </div>
  );
};

export default Tabs;
