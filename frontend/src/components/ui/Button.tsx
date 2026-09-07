import React from 'react';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'destructive' | 'success';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
}

export const Button: React.FC<ButtonProps> = ({
  variant = 'primary',
  size = 'md',
  loading = false,
  disabled,
  className = '',
  children,
  ...props
}) => {
  const variantClass = {
    primary: 'ui-btn-primary',
    secondary: 'ui-btn-secondary',
    ghost: 'ui-btn-ghost',
    destructive: 'ui-btn-destructive',
    success: 'ui-btn-success',
  }[variant];

  const sizeClass = {
    sm: 'ui-btn-sm',
    md: '',
    lg: 'ui-btn-lg',
  }[size];

  return (
    <button
      className={`ui-btn ${variantClass} ${sizeClass} ${className}`.trim()}
      disabled={disabled || loading}
      {...props}
    >
      {loading && <span className="ui-btn-spinner" aria-hidden="true" />}
      <span className={loading ? 'ui-btn-loading-text' : ''}>{children}</span>
    </button>
  );
};
