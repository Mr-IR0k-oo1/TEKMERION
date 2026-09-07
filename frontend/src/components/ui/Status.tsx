import React from 'react';

export interface StatusProps extends React.HTMLAttributes<HTMLDivElement> {
  variant?: 'online' | 'offline' | 'warning' | 'error';
  label: string;
}

export const Status: React.FC<StatusProps> = ({
  variant = 'online',
  label,
  className = '',
  ...props
}) => {
  const variantClass = {
    online: 'ui-status-online',
    offline: 'ui-status-offline',
    warning: 'ui-status-warning',
    error: 'ui-status-error',
  }[variant];

  return (
    <div className={`ui-status ${variantClass} ${className}`.trim()} {...props}>
      <span className="ui-status-dot" />
      <span className="ui-status-label">{label}</span>
    </div>
  );
};
