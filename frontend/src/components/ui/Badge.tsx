import React from 'react';

export interface BadgeProps extends React.HTMLAttributes<HTMLSpanElement> {
  variant?: 'default' | 'success' | 'warning' | 'error' | 'info' | 'purple';
  dot?: boolean;
}

export const Badge: React.FC<BadgeProps> = ({
  variant = 'default',
  dot = false,
  className = '',
  children,
  ...props
}) => {
  const variantClass = {
    default: 'ui-badge-default',
    success: 'ui-badge-success',
    warning: 'ui-badge-warning',
    error: 'ui-badge-error',
    info: 'ui-badge-info',
    purple: 'ui-badge-purple',
  }[variant];

  return (
    <span className={`ui-badge ${variantClass} ${className}`.trim()} {...props}>
      {dot && <span className="ui-badge-dot" />}
      {children}
    </span>
  );
};
