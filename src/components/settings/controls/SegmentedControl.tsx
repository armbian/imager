// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { ReactNode } from 'react';
import { useSettled } from '../../../hooks/useSettled';

export interface SegmentedOption<T extends string> {
  value: T;
  label: string;
  icon?: ReactNode;
}

interface SegmentedControlProps<T extends string> {
  /** null: nothing picked yet, no option pressed */
  value: T | null;
  options: readonly SegmentedOption<T>[];
  onChange: (value: T) => void;
  ariaLabel: string;
  disabled?: boolean;
  /** Stored value not read yet: rendered inert and without transitions */
  loading?: boolean;
}

export function SegmentedControl<T extends string>({ value, options, onChange, ariaLabel, disabled = false, loading = false }: SegmentedControlProps<T>) {
  const settled = useSettled(loading);
  return (
    <div className={`set-seg${disabled || loading ? ' is-disabled' : ''}${settled ? '' : ' is-static'}`} role="group" aria-label={ariaLabel}>
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          className="set-seg__opt"
          aria-pressed={o.value === value}
          disabled={disabled || loading}
          onClick={() => {
            if (o.value !== value) onChange(o.value);
          }}
        >
          {o.icon}
          <span className="set-seg__label">{o.label}</span>
        </button>
      ))}
    </div>
  );
}
