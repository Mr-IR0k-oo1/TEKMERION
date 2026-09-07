import React from 'react';
import { PIPELINE_STAGES, PipelineStageId, PipelineStatus } from '../types/forensic';
import { CheckCircle2, Clock, AlertCircle } from 'lucide-react';

interface StageTrackerProps {
  currentStage: PipelineStageId;
  completedStages: PipelineStageId[];
  status: PipelineStatus;
  onSelectStage?: (stageId: PipelineStageId) => void;
}

export const StageTracker: React.FC<StageTrackerProps> = ({
  currentStage,
  completedStages,
  status,
  onSelectStage,
}) => {
  return (
    <div className="stage-tracker-card">
      <div className="stage-tracker-steps">
        {PIPELINE_STAGES.map((stage, idx) => {
          const isCurrent = currentStage === stage.id && status !== 'completed';
          const isCompleted = completedStages.includes(stage.id) || status === 'completed';
          const isTampered = status === 'tampered' && (stage.id === 'EVIDENCE' || stage.id === 'FINAL_VERIFY');

          let stepClass = 'stage-step';
          if (isTampered) stepClass += ' tampered';
          else if (isCurrent) stepClass += ' active';
          else if (isCompleted) stepClass += ' completed';

          return (
            <div
              key={stage.id}
              className={stepClass}
              onClick={() => onSelectStage && onSelectStage(stage.id)}
              style={{ cursor: onSelectStage ? 'pointer' : 'default' }}
            >
              <div className="stage-header">
                <span className="stage-number">0{idx + 1}</span>
                {isTampered ? (
                  <AlertCircle size={14} color="var(--crimson-tamper)" />
                ) : isCompleted ? (
                  <CheckCircle2 size={14} color="var(--emerald-verified)" />
                ) : isCurrent ? (
                  <span className="pulse-dot" style={{ color: 'var(--cyan-bright)' }} />
                ) : (
                  <Clock size={14} color="var(--text-muted)" />
                )}
              </div>

              <div className="stage-title">{stage.name}</div>
              <div className="stage-desc">{stage.description}</div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
