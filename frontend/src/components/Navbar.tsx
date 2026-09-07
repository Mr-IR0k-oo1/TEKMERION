import React, { useState } from 'react';
import { ViewTab } from '../types/forensic';
import { ShieldCheck, AlertTriangle, Check, Copy } from 'lucide-react';

interface NavbarProps {
  activeTab: ViewTab;
  onSelectTab: (tab: ViewTab) => void;
  runId: string;
  isTampered: boolean;
  contractAddress: string;
  backendOnline?: boolean;
}

const TAB_FLAGS: Record<ViewTab, string> = {
  pipeline:   '--pipeline',
  merkle:     '--merkle',
  tamper:     '--tamper',
  candidates: '--candidates',
  audit:      '--audit',
};

const TAB_LABELS: Record<ViewTab, string> = {
  pipeline:   'Pipeline Studio',
  merkle:     'Merkle Tree',
  tamper:     'Tamper Lab',
  candidates: 'Candidates',
  audit:      'Audit',
};

export const Navbar: React.FC<NavbarProps> = ({
  activeTab,
  onSelectTab,
  runId,
  isTampered,
  contractAddress,
  backendOnline = false,
}) => {
  const [copied, setCopied] = useState(false);

  const handleCopyRunId = () => {
    navigator.clipboard.writeText(runId);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <header className="nav-term-header">
      {/* Compact telemetry strip */}
      <div className="telemetry-strip">
        <div className="telemetry-strip__left">
          <span className="brand-mark">
            <ShieldCheck size={15} />
            <span className="brand-name">TEKMERION</span>
          </span>
          <span className="telemetry-sep" />
          <span className="telemetry-chip">
            <span className="pulse-dot" style={{ color: backendOnline ? 'var(--color-emerald)' : 'var(--color-amber)' }} />
            <span className="telemetry-chip__label">Backend</span>
            <span className="mono" style={{ color: backendOnline ? 'var(--color-emerald)' : 'var(--color-amber)' }}>
              {backendOnline ? 'ONLINE' : 'LOCAL'}
            </span>
          </span>
          <span className="telemetry-chip">
            <span className="pulse-dot" style={{ color: 'var(--color-violet)' }} />
            <span className="telemetry-chip__label">Chain</span>
            <span className="mono" style={{ color: 'var(--color-violet)' }}>Sepolia</span>
          </span>
          <span className="telemetry-chip" title={contractAddress}>
            <span className="telemetry-chip__label">Contract</span>
            <span className="mono">{contractAddress.slice(0, 6)}…{contractAddress.slice(-4)}</span>
          </span>
          <span className="telemetry-chip">
            <span className="telemetry-chip__label">Run</span>
            <button onClick={handleCopyRunId} className="run-id-btn mono">
              {runId}
              {copied
                ? <Check size={11} style={{ color: 'var(--color-emerald)' }} />
                : <Copy size={11} style={{ opacity: 0.5 }} />}
            </button>
          </span>
        </div>

        {isTampered && (
          <div className="tamper-alert-strip">
            <AlertTriangle size={13} />
            <span>TAMPER DETECTED</span>
          </div>
        )}
      </div>

      {/* Terminal command nav */}
      <nav className="nav-term">
        <div className="nav-term__line">
          <span className="nav-term__prompt">&gt;</span>
          <span className="nav-term__prog">tekmerion</span>
          {(Object.keys(TAB_FLAGS) as ViewTab[]).map((tab) => (
            <button
              key={tab}
              className={`nav-term__flag ${activeTab === tab ? 'is-active' : ''} ${tab === 'tamper' && isTampered ? 'is-alert' : ''}`}
              onClick={() => onSelectTab(tab)}
            >
              {TAB_FLAGS[tab]}
            </button>
          ))}
          <span className="nav-term__caret" aria-hidden="true">▮</span>
        </div>
      </nav>
    </header>
  );
};
