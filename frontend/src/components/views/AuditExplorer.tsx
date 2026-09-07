import React, { useState } from 'react';
import { AuditEvent } from '../../types/forensic';
import { BundleExportData, downloadForensicJson, downloadForensicZip } from '../../services/exportBundle';
import {
  Terminal,
  FolderArchive,
  FileJson,
  Download,
  Folder,
  FileText,
  Copy,
  Check,
} from 'lucide-react';

interface AuditExplorerProps {
  exportData: BundleExportData;
  auditEvents: AuditEvent[];
}

export const AuditExplorer: React.FC<AuditExplorerProps> = ({
  exportData,
  auditEvents,
}) => {
  const [selectedFile, setSelectedFile] = useState<string>('evidence/root.json');
  const [copied, setCopied] = useState(false);
  const [isZipping, setIsZipping] = useState(false);

  const getFileContent = (path: string): string => {
    switch (path) {
      case 'input/input_metadata.json':
        return JSON.stringify(
          {
            name: exportData.imageFileName,
            resolution: exportData.resolution,
            sha256: exportData.imageHash,
            run_id: exportData.runId,
            recorded_at: new Date().toISOString(),
          },
          null,
          2
        );
      case 'discovery/candidates.json':
        return JSON.stringify(exportData.candidates.map((c) => c.candidate), null, 2);
      case 'verification/results.json':
        return JSON.stringify(exportData.candidates, null, 2);
      case 'evidence/evidence.json':
        return JSON.stringify(exportData.evidenceRecord, null, 2);
      case 'evidence/leaves.json':
        return JSON.stringify(exportData.evidenceBundle?.tree.leaves, null, 2);
      case 'evidence/root.json':
        return JSON.stringify(
          {
            root_hash: exportData.evidenceBundle?.root_hash,
            generated_at: new Date().toISOString(),
          },
          null,
          2
        );
      case 'blockchain/transaction.json':
        return JSON.stringify(exportData.blockchainRecord, null, 2);
      case 'audit.jsonl':
        return auditEvents.map((e) => JSON.stringify(e)).join('\n');
      default:
        return '// Select a file from the tree to inspect forensic content';
    }
  };

  const handleDownloadZip = async () => {
    try {
      setIsZipping(true);
      await downloadForensicZip(exportData);
    } finally {
      setIsZipping(false);
    }
  };

  const handleDownloadJson = () => {
    downloadForensicJson(exportData);
  };

  const handleCopyContent = () => {
    navigator.clipboard.writeText(getFileContent(selectedFile));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="audit-container">
      {/* Header & Download Actions */}
      <div className="card" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '16px' }}>
        <div>
          <h2 style={{ fontSize: '18px', fontWeight: 800, display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Terminal size={20} color="var(--cyan-bright)" /> Forensic Run Persistence & Audit Log
          </h2>
          <p style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
            Permanent forensic records compliant with TEKMERION Section 16 & 17 specifications.
          </p>
        </div>

        <div style={{ display: 'flex', gap: '10px' }}>
          <button className="btn btn-primary" onClick={handleDownloadZip} disabled={isZipping}>
            <Download size={15} /> {isZipping ? 'Packaging...' : 'Download Run Bundle (.zip)'}
          </button>
          <button className="btn btn-secondary" onClick={handleDownloadJson}>
            <FileJson size={15} /> Export JSON
          </button>
        </div>
      </div>

      <div className="audit-layout">
        {/* Left Column: Forensic Run Bundle Tree */}
        <div className="bundle-files-card">
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <h3 style={{ fontSize: '15px', fontWeight: 700, display: 'flex', alignItems: 'center', gap: '6px' }}>
              <FolderArchive size={16} color="var(--cyan-bright)" /> runs/{exportData.runId}/
            </h3>
          </div>

          <div className="bundle-tree">
            {/* input/ */}
            <div className="bundle-dir">
              <Folder size={14} style={{ display: 'inline', marginRight: '6px' }} />
              input/
            </div>
            <div
              className={`bundle-item ${selectedFile === 'input/input_metadata.json' ? 'active' : ''}`}
              style={{ paddingLeft: '20px' }}
              onClick={() => setSelectedFile('input/input_metadata.json')}
            >
              <FileText size={13} /> input_metadata.json
            </div>

            {/* discovery/ */}
            <div className="bundle-dir" style={{ marginTop: '6px' }}>
              <Folder size={14} style={{ display: 'inline', marginRight: '6px' }} />
              discovery/
            </div>
            <div
              className={`bundle-item ${selectedFile === 'discovery/candidates.json' ? 'active' : ''}`}
              style={{ paddingLeft: '20px' }}
              onClick={() => setSelectedFile('discovery/candidates.json')}
            >
              <FileText size={13} /> candidates.json
            </div>

            {/* verification/ */}
            <div className="bundle-dir" style={{ marginTop: '6px' }}>
              <Folder size={14} style={{ display: 'inline', marginRight: '6px' }} />
              verification/
            </div>
            <div
              className={`bundle-item ${selectedFile === 'verification/results.json' ? 'active' : ''}`}
              style={{ paddingLeft: '20px' }}
              onClick={() => setSelectedFile('verification/results.json')}
            >
              <FileText size={13} /> results.json
            </div>

            {/* evidence/ */}
            <div className="bundle-dir" style={{ marginTop: '6px' }}>
              <Folder size={14} style={{ display: 'inline', marginRight: '6px' }} />
              evidence/
            </div>
            <div
              className={`bundle-item ${selectedFile === 'evidence/evidence.json' ? 'active' : ''}`}
              style={{ paddingLeft: '20px' }}
              onClick={() => setSelectedFile('evidence/evidence.json')}
            >
              <FileText size={13} /> evidence.json
            </div>
            <div
              className={`bundle-item ${selectedFile === 'evidence/leaves.json' ? 'active' : ''}`}
              style={{ paddingLeft: '20px' }}
              onClick={() => setSelectedFile('evidence/leaves.json')}
            >
              <FileText size={13} /> leaves.json
            </div>
            <div
              className={`bundle-item ${selectedFile === 'evidence/root.json' ? 'active' : ''}`}
              style={{ paddingLeft: '20px' }}
              onClick={() => setSelectedFile('evidence/root.json')}
            >
              <FileText size={13} /> root.json
            </div>

            {/* blockchain/ */}
            <div className="bundle-dir" style={{ marginTop: '6px' }}>
              <Folder size={14} style={{ display: 'inline', marginRight: '6px' }} />
              blockchain/
            </div>
            <div
              className={`bundle-item ${selectedFile === 'blockchain/transaction.json' ? 'active' : ''}`}
              style={{ paddingLeft: '20px' }}
              onClick={() => setSelectedFile('blockchain/transaction.json')}
            >
              <FileText size={13} /> transaction.json
            </div>

            {/* audit.jsonl */}
            <div
              className={`bundle-item ${selectedFile === 'audit.jsonl' ? 'active' : ''}`}
              style={{ marginTop: '8px' }}
              onClick={() => setSelectedFile('audit.jsonl')}
            >
              <FileText size={13} /> audit.jsonl
            </div>
          </div>
        </div>

        {/* Right Column: File Content Inspector or Terminal View */}
        <div className="audit-terminal-card">
          <div className="terminal-header">
            <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
              <div className="terminal-dots">
                <span className="dot dot-red" />
                <span className="dot dot-yellow" />
                <span className="dot dot-green" />
              </div>
              <span className="mono" style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>
                {selectedFile}
              </span>
            </div>

            <button
              className="btn btn-secondary"
              style={{ padding: '4px 8px', fontSize: '11px' }}
              onClick={handleCopyContent}
            >
              {copied ? <Check size={13} color="var(--emerald-verified)" /> : <Copy size={13} />} Copy
            </button>
          </div>

          {selectedFile === 'audit.jsonl' ? (
            <div className="terminal-body">
              {auditEvents.map((ev) => (
                <div key={ev.id} className="terminal-line">
                  <span className="log-ts">[{ev.timestamp.split('T')[1].replace('Z', '')}]</span>
                  <span
                    className={
                      ev.level === 'success'
                        ? 'log-msg-success'
                        : ev.level === 'warn'
                        ? 'log-msg-warn'
                        : ev.level === 'error'
                        ? 'log-msg-error'
                        : 'log-msg-info'
                    }
                  >
                    {ev.event}
                  </span>
                </div>
              ))}
            </div>
          ) : (
            <pre
              className="mono"
              style={{
                padding: '18px',
                fontSize: '12px',
                lineHeight: 1.6,
                color: '#38bdf8',
                overflowX: 'auto',
                maxHeight: '520px',
              }}
            >
              {getFileContent(selectedFile)}
            </pre>
          )}
        </div>
      </div>
    </div>
  );
};
