import React, { useState } from 'react';
import { ViewTab } from '../types/forensic';
import { Check, Copy, Search, Briefcase, FileSearch, ShieldCheck, Settings } from 'lucide-react';

interface NavbarProps {
  activeTab: ViewTab;
  onSelectTab: (tab: ViewTab) => void;
  runId: string;
  isTampered: boolean;
  contractAddress: string;
  backendOnline?: boolean;
}

const NAV_ITEMS: { tab: ViewTab; label: string; icon: React.ElementType }[] = [
  { tab: 'pipeline', label: 'Investigate', icon: Search },
  { tab: 'candidates', label: 'Cases', icon: Briefcase },
  { tab: 'evidence', label: 'Evidence', icon: FileSearch },
  { tab: 'audit', label: 'Audit', icon: ShieldCheck },
];

export const Navbar: React.FC<NavbarProps> = ({
  activeTab,
  onSelectTab,
  runId,
  isTampered,
  contractAddress,
  backendOnline = false,
}) => {
  const [copied, setCopied] = useState(false);

  const isActive = (tab: ViewTab) =>
    tab === 'pipeline' || tab === 'candidates' || tab === 'audit'
      ? activeTab === tab
      : activeTab === 'evidence' || activeTab === 'merkle' || activeTab === 'blockchain' || activeTab === 'tamper';

  const handleCopyRunId = () => {
    navigator.clipboard.writeText(runId);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <header className="app-nav">
      <div className="app-nav-brand">
        <span className="app-nav-name">TEKMERION</span>
      </div>

      <nav className="app-nav-tabs" aria-label="Sections">
        {NAV_ITEMS.map((item) => {
          const Icon = item.icon;
          return (
            <button
              key={item.tab}
              className={`app-nav-tab ${isActive(item.tab) ? 'is-active' : ''}`}
              onClick={() => onSelectTab(item.tab)}
            >
              <Icon size={14} strokeWidth={2} />
              {item.label}
              {item.tab === 'evidence' && isTampered && <span className="app-nav-alert-dot" />}
            </button>
          );
        })}
      </nav>

      <div className="app-nav-status">
        <span className="app-status-chip" title={contractAddress}>
          <span className={`app-status-dot ${backendOnline ? 'is-online' : 'is-local'}`} />
          <span>{backendOnline ? 'Backend Online' : 'Backend Local'}</span>
        </span>
        <span className="app-status-sep" />
        <button className="app-run-id mono" onClick={handleCopyRunId} aria-label="Copy run ID">
          {runId}
          {copied ? <Check size={11} strokeWidth={2.5} /> : <Copy size={11} strokeWidth={1.75} />}
        </button>
        <span className="app-status-sep" />
        <button className="app-settings-btn" aria-label="Settings">
          <Settings size={14} strokeWidth={1.75} />
        </button>
      </div>
    </header>
  );
};