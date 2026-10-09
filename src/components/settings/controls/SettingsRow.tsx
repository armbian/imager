// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { MouseEvent, ReactNode } from 'react';

interface SettingsRowProps {
  label: string;
  description?: ReactNode;
  control: ReactNode;
  tall?: boolean;
  danger?: boolean;
  onClick?: (event: MouseEvent<HTMLButtonElement>) => void;
  trailing?: ReactNode;
  /** Id of the form control in `control`, so the label becomes its <label>. */
  controlId?: string;
  descriptionId?: string;
}

export function SettingsRow({ label, description, control, tall = false, danger = false, onClick, trailing, controlId, descriptionId }: SettingsRowProps) {
  const className = `set-row${tall ? ' set-row--tall' : ''}${danger ? ' set-row--danger' : ''}${onClick ? ' set-row--clickable' : ''}`;
  const text = (
    <span className="set-row__text">
      {controlId ? (
        <label className="set-row__label" htmlFor={controlId}>
          {label}
        </label>
      ) : (
        <span className="set-row__label">{label}</span>
      )}
      {description && (
        <span className="set-row__desc" id={descriptionId}>
          {description}
        </span>
      )}
    </span>
  );
  const end = (
    <span className="set-row__control">
      {control}
      {trailing && <span className="set-row__trailing">{trailing}</span>}
    </span>
  );

  // A clickable row is one button, so `control` and `trailing` must not be interactive there.
  if (onClick) {
    return (
      <button type="button" className={className} onClick={onClick}>
        {text}
        {end}
      </button>
    );
  }
  return (
    <div className={className}>
      {text}
      {end}
    </div>
  );
}
