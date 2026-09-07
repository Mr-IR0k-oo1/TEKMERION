import React from 'react';

export interface ProgressProps extends React.HTMLAttributes<HTMLDivElement> {
  value: number;
  variant?: 'default' | 'success' | 'warning' | 'error';
  size?: 'sm' | 'md';
  showLabel?: boolean;
}

export const Progress: React.FC<ProgressProps> = ({
  value,
  variant = 'default',
  size = 'md',
  showLabel = false,
  className = '',
  ...props
}) => {
  const clamped = Math.max(0, Math.min(100, value));

  const variantClass = {
    default: 'ui-progress-default',
    success: 'ui-progress-success',
    warning: 'ui-progress-warning',
    error: 'ui-progress-error',
  }[variant];

  const sizeClass = {
    sm: 'ui-progress-sm',
    md: '',
  }[size];

  return (
    <div className={`ui-progress-wrapper ${sizeClass} ${className}`.trim()} {...props}>
      <div className="ui-progress-track">
        <div
          className={`ui-progress-fill ${variantClass}`}
          style={{ width: `${clamped}%` }}
          role="progressbar"
          aria-valuenow={clamped}
          aria-valuemin={0}
          aria-valuemax={100}
        />
      </div>
      {showLabel && <span className="ui-progress-label">{Math.round(clamped)}%</span>}
    </div>
  );
};
