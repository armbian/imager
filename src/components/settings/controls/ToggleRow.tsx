// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useId, type ReactNode } from 'react';
import { useSettled } from '../../../hooks/useSettled';
import { SettingsRow } from './SettingsRow';

interface ToggleRowProps {
  label: string;
  description?: ReactNode;
  checked: boolean;
  onChange: (value: boolean) => void;
  disabled?: boolean;
  busy?: boolean;
  /** Stored value not read yet: rendered inert and without transitions */
  loading?: boolean;
  danger?: boolean;
  tall?: boolean;
}

export function ToggleRow({ label, description, checked, onChange, disabled = false, busy = false, loading = false, danger, tall }: ToggleRowProps) {
  const id = useId();
  const settled = useSettled(loading);
  const descriptionId = description ? `${id}-desc` : undefined;
  return (
    <SettingsRow
      label={label}
      description={description}
      danger={danger}
      tall={tall}
      controlId={id}
      descriptionId={descriptionId}
      control={
        <label className={`toggle-switch${disabled || loading ? ' is-disabled' : ''}${settled ? '' : ' is-static'}`}>
          <input
            id={id}
            type="checkbox"
            role="switch"
            checked={checked}
            disabled={disabled || loading}
            aria-busy={busy || undefined}
            aria-describedby={descriptionId}
            // Busy keeps the input enabled so a save in flight does not drop keyboard focus.
            onChange={(e) => {
              if (!busy) onChange(e.target.checked);
            }}
          />
          <span className="toggle-slider" aria-hidden="true" />
        </label>
      }
    />
  );
}
