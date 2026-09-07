import React, { useState } from 'react';
import { BlockchainRecord } from '../../types/forensic';
import { ShieldCheck, Copy, Check } from 'lucide-react';
import { Badge } from '../ui';

interface BlockchainProofProps {
  blockchainRecord: BlockchainRecord | null;
  verified: boolean;
}

const trunc = (s: string, pad = 10) =>
  `${s.substring(0, pad)}…${s.substring(s.length - 8)}`;

export const BlockchainProof: React.FC<BlockchainProofProps> = ({
  blockchainRecord,
  verified,
}) => {
  const [copied, setCopied] = useState<string | null>(null);

  if (!blockchainRecord) {
    return (
      <div className="proof-empty">
        <div className="proof-empty-icon">
          <ShieldCheck size={28} strokeWidth={1.5} />
        </div>
        <h2 className="proof-empty-title">No anchor yet</h2>
        <p className="proof-empty-text">
          Run the verification pipeline to register this record's fingerprint on-chain.
        </p>
      </div>
    );
  }

  const copy = (value: string) => {
    navigator.clipboard.writeText(value);
    setCopied(value);
    setTimeout(() => setCopied(null), 2000);
  };

  const timestamp = new Date(blockchainRecord.timestamp).toLocaleString();

  const rows: { label: string; value: string; kind?: 'copy' }[] = [
    { label: 'Network', value: 'Ethereum Sepolia' },
    { label: 'Root Hash', value: blockchainRecord.registered_root, kind: 'copy' },
    { label: 'Transaction', value: blockchainRecord.tx_hash, kind: 'copy' },
    { label: 'Block', value: `#${blockchainRecord.block_number.toLocaleString()}` },
    { label: 'Confirmations', value: String(blockchainRecord.confirmations) },
    { label: 'Timestamp', value: timestamp },
  ];

  return (
    <div className="blockchain-view">
      <header className="proof-header">
        <div>
          <h1 className="proof-title">Blockchain Proof</h1>
          <p className="proof-subtitle">
            The fingerprint was recorded in a public ledger so it can be checked at any time.
          </p>
        </div>
        <Badge variant={verified ? 'success' : 'error'} dot>
          {verified ? 'Verified' : 'Not Verified'}
        </Badge>
      </header>

      <div className="blockchain-card">
        <div className="blockchain-label-row">
          <span className="blockchain-label">Network</span>
          <span className="blockchain-value">Ethereum Sepolia</span>
        </div>

        <div className="blockchain-divider" />

        {rows.slice(1).map((row) => (
          <div className="blockchain-label-row" key={row.label}>
            <span className="blockchain-label">{row.label}</span>
            <span className="blockchain-value">
              <span className={`blockchain-value-text ${row.kind ? 'mono blockchain-ink' : ''}`}>
                {row.kind ? trunc(row.value) : row.value}
              </span>
              {row.kind && (
                <button
                  className="proof-copy-btn"
                  onClick={() => copy(row.value)}
                  aria-label={`Copy ${row.label}`}
                >
                  {copied === row.value ? <Check size={13} /> : <Copy size={13} />}
                </button>
              )}
            </span>
          </div>
        ))}

        <div className="blockchain-divider" />

        <div className="blockchain-proof-note">
          <span className="blockchain-proof-dot" />
          <p>
            The registered blockchain record contains this evidence fingerprint.
          </p>
        </div>
      </div>

      <p className="blockchain-explainer">
        This confirms the fingerprint was included on the Sepolia ledger at the time shown. It verifies the
        record itself — it does not establish who is in the image.
      </p>
    </div>
  );
};