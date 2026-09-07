import React, { useState } from 'react';
import { VerificationResult } from '../../types/forensic';
import { Sheet } from '../ui';
import { CandidateComparison } from './CandidateComparison';
import { ExternalLink } from 'lucide-react';

interface DiscoveryProps {
  candidates: VerificationResult[];
  queryImageSrc: string;
}

const statusLabel: Record<string, { text: string; tone: 'success' | 'warning' | 'error' | 'default' }> = {
  verified: { text: 'Verified', tone: 'success' },
  below_threshold: { text: 'Low similarity', tone: 'warning' },
  no_face: { text: 'No face', tone: 'error' },
  error: { text: 'Error', tone: 'default' },
};

export const Discovery: React.FC<DiscoveryProps> = ({ candidates, queryImageSrc }) => {
  const [selected, setSelected] = useState<VerificationResult | null>(null);

  return (
    <div className="discovery">
      <div className="discovery-header">
        <div>
          <span className="discovery-kicker">Discovery</span>
          <h2 className="discovery-title">{candidates.length} result{candidates.length === 1 ? '' : 's'}</h2>
        </div>
        {candidates.length === 0 && (
          <p className="discovery-empty-note">
            Candidate results will appear here once the investigation finds matches.
          </p>
        )}
      </div>

      <div className="discovery-results">
        {candidates.map((item) => {
          const { candidate } = item;
          const label = statusLabel[item.status] || statusLabel.error;
          return (
            <button
              key={candidate.url}
              className="discovery-result"
              onClick={() => setSelected(item)}
            >
              <div className="discovery-result-media">
                <img
                  src={candidate.thumbnail_url || candidate.image_url}
                  alt={candidate.title}
                  loading="lazy"
                  onError={(e) => {
                    (e.target as HTMLImageElement).style.display = 'none';
                  }}
                />
              </div>

              <div className="discovery-result-body">
                <div className="discovery-result-domain">{candidate.domain}</div>
                <div className="discovery-result-url">{candidate.url}</div>

                <div className="discovery-result-footer">
                  <span className={`discovery-status discovery-status-${label.tone}`}>
                    <span className="discovery-status-dot" />
                    {label.text}
                  </span>
                  <span className="discovery-similarity">
                    Embedding similarity: {item.similarity.toFixed(3)}
                  </span>
                </div>
              </div>
            </button>
          );
        })}
      </div>

      <Sheet
        isOpen={!!selected}
        onClose={() => setSelected(null)}
        title="Candidate Comparison"
        side="right"
      >
        {selected && (
          <div className="discovery-sheet">
            <a
              className="discovery-sheet-link"
              href={selected.candidate.url}
              target="_blank"
              rel="noreferrer"
            >
              <ExternalLink size={14} />
              <span>{selected.candidate.domain}</span>
            </a>
            <CandidateComparison
              queryImageSrc={queryImageSrc}
              candidate={selected}
            />
          </div>
        )}
      </Sheet>
    </div>
  );
};
