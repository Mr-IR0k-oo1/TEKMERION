import React from 'react';

export interface DividerProps extends React.HTMLAttributes<HTMLHRElement> {
  vertical?: boolean;
  spaced?: boolean;
}

export const Divider: React.FC<DividerProps> = ({
  vertical = false,
  spaced = false,
  className = '',
  ...props
}) => {
  if (vertical) {
    return <span className={`ui-divider-vertical ${spaced ? 'ui-divider-spaced' : ''} ${className}`.trim()} role="separator" aria-orientation="vertical" {...props} />;
  }
  return <hr className={`ui-divider ${spaced ? 'ui-divider-spaced' : ''} ${className}`.trim()} {...props} />;
};
