import React, { useState } from 'react';
import { ViewTab } from '../types/forensic';
import { ShieldCheck, Cpu, GitFork, AlertTriangle, Users, Terminal, Check, Copy } from 'lucide-react';

interface NavbarProps {
  activeTab: ViewTab;
  onSelectTab: (tab: ViewTab) => void;
  runId: string;
  isTampered: boolean;
  contractAddress: string;
  backendOnline?: boolean;
}

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
    <header>
      {/* Top Telemetry Bar */}
      <div className="telemetry-bar">
        <div className="brand-section">
          <div className="brand-logo-icon">
            <ShieldCheck size={22} />
          </div>
          <div className="brand-title-group">
            <h1>TEKMERION</h1>
            <p>Forensic Evidence Verification Engine</p>
          </div>
        </div>

        <div className="telemetry-stats">
          <div className="stat-item">
            <span className="stat-label">Backend Worker</span>
            <span
              className="stat-value"
              style={{ color: backendOnline ? 'var(--emerald-verified)' : 'var(--amber-warn)' }}
            >
              <span className="pulse-dot" /> {backendOnline ? 'ONLINE' : 'LOCAL'}
            </span>
          </div>

          <div className="stat-item">
            <span className="stat-label">Network Anchor</span>
            <span className="stat-value" style={{ color: 'var(--violet-chain)' }}>
              <span className="pulse-dot" /> Ethereum Sepolia
            </span>
          </div>

          <div className="stat-item" title={contractAddress}>
            <span className="stat-label">Registry Contract</span>
            <span className="stat-value mono" style={{ fontSize: '12px' }}>
              {contractAddress.substring(0, 6)}...{contractAddress.substring(contractAddress.length - 4)}
            </span>
          </div>

          <div className="stat-item">
            <span className="stat-label">Active Run ID</span>
            <button
              onClick={handleCopyRunId}
              className="stat-value mono btn-secondary"
              style={{
                padding: '4px 10px',
                fontSize: '12px',
                borderRadius: '4px',
                display: 'inline-flex',
                alignItems: 'center',
                gap: '6px',
                cursor: 'pointer',
              }}
            >
              {runId}
              {copied ? <Check size={13} color="var(--emerald-verified)" /> : <Copy size={13} />}
            </button>
          </div>

          {isTampered && (
            <div className="badge badge-crimson" style={{ animation: 'flashRed 1.8s infinite' }}>
              <AlertTriangle size={13} /> TAMPER DETECTED
            </div>
          )}
        </div>
      </div>

      {/* Navigation Tabs */}
      <nav className="view-tabs-container">
        <button
          className={`view-tab-btn ${activeTab === 'pipeline' ? 'active' : ''}`}
          onClick={() => onSelectTab('pipeline')}
        >
          <Cpu size={16} /> Pipeline Studio
        </button>

        <button
          className={`view-tab-btn ${activeTab === 'merkle' ? 'active' : ''}`}
          onClick={() => onSelectTab('merkle')}
        >
          <GitFork size={16} /> Merkle Tree
        </button>

        <button
          className={`view-tab-btn ${activeTab === 'tamper' ? 'active' : ''} ${
            isTampered ? 'tamper-active' : ''
          }`}
          onClick={() => onSelectTab('tamper')}
        >
          <AlertTriangle size={16} /> Tamper Lab
          {isTampered && <span className="badge badge-crimson" style={{ padding: '1px 6px', fontSize: '10px' }}>ALERT</span>}
        </button>

        <button
          className={`view-tab-btn ${activeTab === 'candidates' ? 'active' : ''}`}
          onClick={() => onSelectTab('candidates')}
        >
          <Users size={16} /> Candidate Inspector
        </button>

        <button
          className={`view-tab-btn ${activeTab === 'audit' ? 'active' : ''}`}
          onClick={() => onSelectTab('audit')}
        >
          <Terminal size={16} /> Audit Explorer
        </button>
      </nav>
    </header>
  );
};
