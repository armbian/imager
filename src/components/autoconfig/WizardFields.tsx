// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { ReactNode } from 'react';
import { ChevronDown, Plus } from 'lucide-react';

export function WizardField({ label, children, hint, htmlFor }: {
  label: string;
  children: ReactNode;
  hint?: ReactNode;
  htmlFor?: string;
}) {
  return (
    <div className="pw-field">
      {htmlFor ? (
        <label className="pw-field__label" htmlFor={htmlFor}>{label}</label>
      ) : (
        <span className="pw-field__label">{label}</span>
      )}
      {children}
      {hint && <div className="pw-field__hint">{hint}</div>}
    </div>
  );
}

export function WizardTextField({ id, value, onChange, icon, mono, placeholder, invalid, autoFocus, onEnter }: {
  id: string;
  value: string;
  onChange: (value: string) => void;
  icon?: ReactNode;
  mono?: boolean;
  placeholder?: string;
  invalid?: boolean;
  autoFocus?: boolean;
  onEnter?: () => void;
}) {
  return (
    <span className={`pe-input${mono ? ' is-mono' : ''}${invalid ? ' is-invalid' : ''}`}>
      {icon}
      <input
        id={id}
        type="text"
        value={value}
        placeholder={placeholder}
        autoFocus={autoFocus}
        autoComplete="off"
        spellCheck={false}
        autoCapitalize="off"
        autoCorrect="off"
        aria-invalid={invalid || undefined}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && onEnter) {
            e.preventDefault();
            onEnter();
          }
        }}
      />
    </span>
  );
}

export function WizardMore({ label, open, onToggle }: { label: string; open: boolean; onToggle: () => void }) {
  return (
    <button type="button" className={`pw-more${open ? ' is-open' : ''}`} aria-expanded={open} onClick={onToggle}>
      {open ? <ChevronDown size={14} aria-hidden="true" /> : <Plus size={14} aria-hidden="true" />}
      {label}
    </button>
  );
}
