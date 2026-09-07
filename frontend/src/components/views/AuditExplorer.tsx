import React from 'react';
import { AuditEvent } from '../../types/forensic';
import { BundleExportData, downloadForensicJson, downloadForensicZip } from '../../services/exportBundle';
import { FileJson, FolderArchive, Activity } from 'lucide-react';

interface AuditExplorerProps {
  exportData: BundleExportData;
  auditEvents: AuditEvent[];
}

const timeOf = (iso: string) =>
  new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });

const levelTone = (level: AuditEvent['level']) => {
  switch (level) {
    case 'success': return 'success';
    case 'warn': return 'warning';
    case 'error': return 'error';
    default: return 'default';
  }
};

export const AuditExplorer: React.FC<AuditExplorerProps> = ({
  exportData,
  auditEvents,
}) => {
  const events = [...auditEvents].reverse();

  return (
    <div className="audit-view">
      <header className="proof-header">
        <div>
          <h1 className="proof-title">Activity</h1>
          <p className="proof-subtitle">
            The chronological record of this verification run from input to on-chain verification.
          </p>
        </div>

        <div className="audit-actions">
          <button className="btn btn-secondary btn-sm" onClick={() => downloadForensicJson(exportData)}>
            <FileJson size={14} /> Export JSON
          </button>
          <button className="btn btn-secondary btn-sm" onClick={() => downloadForensicZip(exportData)}>
            <FolderArchive size={14} /> Download Bundle
          </button>
        </div>
      </header>

      {events.length === 0 ? (
        <div className="proof-empty">
          <div className="proof-empty-icon">
            <Activity size={28} strokeWidth={1.5} />
          </div>
          <h2 className="proof-empty-title">No activity yet</h2>
          <p className="proof-empty-text">
            Run a verification to begin building the activity record.
          </p>
        </div>
      ) : (
        <div className="audit-timeline">
          {events.map((ev, idx) => (
            <div className="audit-row" key={ev.id}>
              <span className="audit-row-time mono">{timeOf(ev.timestamp)}</span>
              <span className={`audit-row-marker audit-row-marker-${levelTone(ev.level)}`}>
                <span className="audit-row-dot" />
                {idx < events.length - 1 && <span className="audit-row-line" />}
              </span>
              <div className="audit-row-content">
                <span className="audit-row-event">{ev.event}</span>
                {ev.detail && <span className="audit-row-detail">{ev.detail}</span>}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};