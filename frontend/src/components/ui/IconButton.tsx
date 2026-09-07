import React from 'react';

export interface IconButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'destructive';
  size?: 'sm' | 'md' | 'lg';
  'aria-label': string;
}

export const IconButton: React.FC<IconButtonProps> = ({
  variant = 'ghost',
  size = 'md',
  className = '',
  children,
  ...props
}) => {
  const variantClass = {
    primary: 'ui-icon-btn-primary',
    secondary: 'ui-icon-btn-secondary',
    ghost: 'ui-icon-btn-ghost',
    destructive: 'ui-icon-btn-destructive',
  }[variant];

  const sizeClass = {
    sm: 'ui-icon-btn-sm',
    md: '',
    lg: 'ui-icon-btn-lg',
  }[size];

  return (
    <button
      className={`ui-icon-btn ${variantClass} ${sizeClass} ${className}`.trim()}
      {...props}
    >
      {children}
    </button>
  );
};
