import React from 'react';

export interface MetricProps extends React.HTMLAttributes<HTMLDivElement> {
  label: string;
  value: string | number;
  unit?: string;
  trend?: 'up' | 'down' | 'neutral';
}

export const Metric: React.FC<MetricProps> = ({
  label,
  value,
  unit,
  trend,
  className = '',
  ...props
}) => {
  return (
    <div className={`ui-metric ${className}`.trim()} {...props}>
      <span className="ui-metric-label">{label}</span>
      <div className="ui-metric-value-row">
        <span className="ui-metric-value">{value}</span>
        {unit && <span className="ui-metric-unit">{unit}</span>}
        {trend && (
          <span className={`ui-metric-trend ui-metric-trend-${trend}`}>
            {trend === 'up' && '\u2191'}
            {trend === 'down' && '\u2193'}
            {trend === 'neutral' && '\u2014'}
          </span>
        )}
      </div>
    </div>
  );
};
