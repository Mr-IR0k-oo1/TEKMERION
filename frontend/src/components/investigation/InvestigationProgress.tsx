import React from 'react';
import { Check, Loader2, ShieldCheck, ShieldAlert, AlertCircle, ArrowLeft } from 'lucide-react';
import { FaceAnalysis, Discovery, CandidateComparison } from '../analysis';
import { Divider } from '../ui';
import type { VerificationResult } from '../../types/forensic';

export type InvestigationStatus = 'idle' | 'running' | 'completed' | 'tampered' | 'error' | 'no_match';

export interface InvestigationStage {
  name: string;
  description: string;
}

export const INVESTIGATION_STAGES: InvestigationStage[] = [
  { name: 'Input', description: 'Ingesting the image' },
  { name: 'Face Analysis', description: 'Detecting and profiling the face' },
  { name: 'Discovery', description: 'Finding candidate sources' },
  { name: 'Verification', description: 'Verifying candidates' },
  { name: 'Match', description: 'Confirming the match' },
  { name: 'Fingerprint', description: 'Generating the evidence fingerprint' },
  { name: 'Blockchain', description: 'Registering on chain' },
  { name: 'Verify', description: 'Confirming the anchor' },
];

interface InvestigationProgressProps {
  fileName: string;
  dimensions: string;
  fileSize: string;
  imageSrc: string;
  runId?: string;
  currentStageIndex: number;
  completedStages: number;
  status: InvestigationStatus;
  stageContext: string;
  onBack: () => void;
  evidenceHash?: string;
  faceQuality?: {
    faceCount: number;
    bbox: [number, number, number, number];
    extractionQuality: number;
    blurVariance: number;
    brightness: number;
    status: 'pass' | 'fail' | 'warn' | 'idle';
  };
  candidates?: VerificationResult[];
}

export const InvestigationProgress: React.FC<InvestigationProgressProps> = ({
  fileName,
  dimensions,
  fileSize,
  imageSrc,
  runId,
  currentStageIndex,
  completedStages,
  status,
  stageContext,
  onBack,
  evidenceHash,
  faceQuality,
  candidates,
}) => {
  const totalStages = INVESTIGATION_STAGES.length;
  const isRunning = status === 'running';
  const isCompleted = status === 'completed';
  const isTampered = status === 'tampered';
  const isError = status === 'error' || status === 'no_match';

  const isFaceStage = currentStageIndex === 1 && !isCompleted && !isError;
  const isDiscoveryStage = currentStageIndex === 2 && !isCompleted && !isError;
  const isCompareStage = (currentStageIndex === 3 || currentStageIndex === 4) && !isCompleted && !isError;

  const shortHash = evidenceHash ? `${evidenceHash.substring(0, 8)}…${evidenceHash.substring(evidenceHash.length - 4)}` : '—';
  const shortRunId = runId ? runId.substring(0, 12) : '—';
  const currentStageName = INVESTIGATION_STAGES[Math.min(currentStageIndex, totalStages - 1)]?.name;

  const renderStatusBadge = () => {
    if (isCompleted) return <span className="badge badge-emerald"><ShieldCheck size={12} /> Verified</span>;
    if (isTampered) return <span className="badge badge-crimson"><ShieldAlert size={12} /> Not Verified</span>;
    if (status === 'no_match') return <span className="badge badge-amber"><AlertCircle size={12} /> No match</span>;
    if (isError) return <span className="badge badge-crimson"><AlertCircle size={12} /> Error</span>;
    return (
      <span className="invest-status-running">
        <span className="invest-status-running-dot" />
        Running
      </span>
    );
  };

  const renderFocus = () => {
    if (isCompleted) {
      return (
        <div className="invest-complete">
          <div className="invest-result is-verified">
            <div className="invest-result-icon">
              <ShieldCheck size={28} strokeWidth={1.5} />
            </div>
            <div className="invest-result-text-wrap">
              <h2 className="invest-result-title">Verified</h2>
              <p className="invest-result-text">
                The evidence fingerprint matches the registered blockchain anchor.
              </p>
            </div>
          </div>

          {(faceQuality || (candidates && candidates.length > 0)) && (
            <div className="invest-findings">
              {faceQuality && faceQuality.faceCount > 0 && (
                <section className="invest-findings-section">
                  <span className="invest-findings-kicker">Face Analysis</span>
                  <FaceAnalysis
                    imageSrc={imageSrc}
                    imageFileName={fileName}
                    resolution={dimensions}
                    faceCount={faceQuality.faceCount}
                    bbox={faceQuality.bbox}
                    extractionQuality={faceQuality.extractionQuality}
                    blurVariance={faceQuality.blurVariance}
                    brightness={faceQuality.brightness}
                    status={faceQuality.status}
                  />
                </section>
              )}

              {candidates && candidates.length > 0 && (
                <section className="invest-findings-section">
                  <span className="invest-findings-kicker">Discovery</span>
                  <Discovery candidates={candidates} queryImageSrc={imageSrc} />
                </section>
              )}

              {candidates && candidates[0] && (
                <section className="invest-findings-section">
                  <span className="invest-findings-kicker">Candidate Comparison</span>
                  <CandidateComparison queryImageSrc={imageSrc} candidate={candidates[0]} />
                </section>
              )}
            </div>
          )}
        </div>
      );
    }

    if (isTampered) {
      return (
        <div className="invest-result is-tampered">
          <div className="invest-result-icon">
            <ShieldAlert size={28} strokeWidth={1.5} />
          </div>
          <div className="invest-result-text-wrap">
            <h2 className="invest-result-title">Evidence has changed</h2>
            <p className="invest-result-text">
              The current fingerprint does not match the registered anchor.
            </p>
          </div>
        </div>
      );
    }

    if (isError) {
      return (
        <div className="invest-result is-error">
          <div className="invest-result-icon">
            <AlertCircle size={28} strokeWidth={1.5} />
          </div>
          <div className="invest-result-text-wrap">
            <h2 className="invest-result-title">
              {status === 'no_match' ? 'No match found' : 'Unable to complete the investigation'}
            </h2>
            <p className="invest-result-text">
              {status === 'no_match'
                ? 'The image could not be matched to a verified source.'
                : 'An unexpected issue occurred. Please try again.'}
            </p>
          </div>
        </div>
      );
    }

    if (isFaceStage) {
      return (
        <div className="invest-analysis">
          {faceQuality && (
            <FaceAnalysis
              imageSrc={imageSrc}
              imageFileName={fileName}
              resolution={dimensions}
              faceCount={faceQuality.faceCount}
              bbox={faceQuality.bbox}
              extractionQuality={faceQuality.extractionQuality}
              blurVariance={faceQuality.blurVariance}
              brightness={faceQuality.brightness}
              status={faceQuality.status}
            />
          )}
        </div>
      );
    }

    if (isDiscoveryStage) {
      return (
        <div className="invest-analysis">
          {candidates && <Discovery candidates={candidates} queryImageSrc={imageSrc} />}
        </div>
      );
    }

    if (isCompareStage) {
      return (
        <div className="invest-analysis">
          {candidates && candidates[0] && (
            <CandidateComparison queryImageSrc={imageSrc} candidate={candidates[0]} />
          )}
        </div>
      );
    }

    return (
      <div className="invest-current">
        <span className="invest-kicker">
          Step {Math.min(currentStageIndex + 1, totalStages)} of {totalStages}
        </span>
        <h2 className="invest-current-title">{currentStageName}</h2>
        <p className="invest-current-context">{stageContext || INVESTIGATION_STAGES[Math.min(currentStageIndex, totalStages - 1)]?.description}</p>
        {isRunning && (
          <div className="invest-busy" aria-hidden="true">
            <Loader2 size={14} className="invest-spin" />
            <span>Working…</span>
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="invest-workspace">
      <div className="invest-toolbar">
        <div className="invest-toolbar-group">
          <button className="invest-back" onClick={onBack} aria-label="Back to start">
            <ArrowLeft size={16} />
          </button>
          <div
            className="invest-thumb"
            style={{ backgroundImage: `url(${imageSrc})` }}
            role="img"
            aria-label={fileName}
          />
          <div className="invest-file-info">
            <span className="invest-file-name">{fileName}</span>
            <span className="invest-file-meta">
              {dimensions} · {fileSize}
            </span>
          </div>
        </div>
        <div className="invest-toolbar-group">{renderStatusBadge()}</div>
      </div>

      <div className="invest-body">
        <aside className="invest-side">
          <span className="invest-section-label">Pipeline</span>
          <nav className="invest-steps" aria-label="Pipeline">
            {INVESTIGATION_STAGES.map((stage, idx) => {
              const isDone = idx < completedStages || isCompleted;
              const isCurrent = idx === currentStageIndex && isRunning;

              return (
                <div key={stage.name} className={`invest-step ${isDone ? 'is-done' : ''} ${isCurrent ? 'is-current' : ''}`}>
                  <div className="invest-step-rail">
                    <span className="invest-step-marker">
                      {isDone ? (
                        <Check size={10} strokeWidth={3} />
                      ) : isCurrent ? (
                        <Loader2 size={10} className="invest-spin" />
                      ) : (
                        <span className="invest-step-dot" />
                      )}
                    </span>
                    {idx < totalStages - 1 && <span className="invest-step-line" />}
                  </div>
                  <div className="invest-step-text">
                    <span className="invest-step-label">{stage.name}</span>
                    <span className="invest-step-desc">{stage.description}</span>
                  </div>
                </div>
              );
            })}
          </nav>
        </aside>

        <section className="invest-main">{renderFocus()}</section>

        <aside className="invest-inspector">
          <div className="invest-panel">
            <div className="invest-panel-title">Evidence</div>
            <div className="invest-row">
              <span className="invest-row-label">Filename</span>
              <span className="invest-row-value">{fileName}</span>
            </div>
            <Divider />
            <div className="invest-row">
              <span className="invest-row-label">Dimensions</span>
              <span className="invest-row-value">{dimensions}</span>
            </div>
            <Divider />
            <div className="invest-row">
              <span className="invest-row-label">File size</span>
              <span className="invest-row-value">{fileSize}</span>
            </div>
            <Divider />
            <div className="invest-row">
              <span className="invest-row-label">SHA-256</span>
              <span className="invest-row-value mono">{shortHash}</span>
            </div>
            <Divider />
            <div className="invest-row">
              <span className="invest-row-label">Status</span>
              <span className="invest-row-value">{renderStatusBadge()}</span>
            </div>
          </div>

          <div className="invest-panel">
            <div className="invest-panel-title">Run</div>
            <div className="invest-row">
              <span className="invest-row-label">Run ID</span>
              <span className="invest-row-value mono">{shortRunId}</span>
            </div>
            <Divider />
            <div className="invest-row">
              <span className="invest-row-label">Stage</span>
              <span className="invest-row-value">{isRunning || !isCompleted ? currentStageName : 'Complete'}</span>
            </div>
            {!isCompleted && !isError && (
              <>
                <Divider />
                <div className="invest-row">
                  <span className="invest-row-label">Context</span>
                  <span className="invest-row-value">{stageContext}</span>
                </div>
              </>
            )}
          </div>
        </aside>
      </div>
    </div>
  );
};