// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { ReactNode } from 'react';

export interface SegmentedOption<T extends string> {
  value: T;
  label: string;
  icon?: ReactNode;
}

interface SegmentedChoiceProps<T extends string> {
  value: T;
  options: readonly SegmentedOption<T>[];
  onChange: (value: T) => void;
  ariaLabel: string;
  size?: 'md' | 'sm';
}

export function SegmentedChoice<T extends string>({ value, options, onChange, ariaLabel, size = 'md' }: SegmentedChoiceProps<T>) {
  return (
    <div
      className={`seg-choice seg-choice--${size}`}
      role="group"
      aria-label={ariaLabel}
      style={{ gridTemplateColumns: `repeat(${options.length}, minmax(0, 1fr))` }}
    >
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          className={`seg-choice__opt${o.value === value ? ' is-on' : ''}`}
          aria-pressed={o.value === value}
          onClick={() => onChange(o.value)}
        >
          {o.icon}
          <span className="seg-choice__label">{o.label}</span>
        </button>
      ))}
    </div>
  );
}
