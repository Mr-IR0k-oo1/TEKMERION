import React from 'react';

export interface Tab {
  id: string;
  label: string;
  icon?: React.ReactNode;
}

export interface TabsProps {
  tabs: Tab[];
  activeTab: string;
  onChange: (id: string) => void;
  className?: string;
}

export const Tabs: React.FC<TabsProps> = ({
  tabs,
  activeTab,
  onChange,
  className = '',
}) => {
  return (
    <div className={`ui-tabs ${className}`.trim()} role="tablist">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          className={`ui-tab ${activeTab === tab.id ? 'ui-tab-active' : ''}`}
          role="tab"
          aria-selected={activeTab === tab.id}
          onClick={() => onChange(tab.id)}
        >
          {tab.icon && <span className="ui-tab-icon">{tab.icon}</span>}
          {tab.label}
        </button>
      ))}
    </div>
  );
};
