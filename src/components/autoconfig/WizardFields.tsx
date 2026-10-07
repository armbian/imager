// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useState, type ReactNode } from 'react';
import { ChevronDown, Globe, Plus, TriangleAlert } from 'lucide-react';
import { UI } from '../../config';
import { loadCountryFlag } from '../../config/countryFlags';

export function WizardField({ label, children, hint, error, status, htmlFor }: {
  label: string;
  children: ReactNode;
  hint?: string;
  error?: string | null;
  status?: ReactNode;
  htmlFor?: string;
}) {
  return (
    <div className="pw-field">
      <label className="pw-field__label" htmlFor={htmlFor}>{label}</label>
      {children}
      {error ? (
        <div className="pw-hint is-error" role="alert">
          <TriangleAlert size={UI.ICON_SIZE.WIZARD_HINT} aria-hidden="true" />
          {error}
        </div>
      ) : status ? (
        status
      ) : hint ? (
        <div className="pw-hint">
          {hint}
        </div>
      ) : null}
    </div>
  );
}

export function WizardTextField({ id, value, onChange, icon, prefix, mono, placeholder, invalid, autoFocus, onEnter }: {
  id: string;
  value: string;
  onChange: (value: string) => void;
  icon?: ReactNode;
  prefix?: string;
  mono?: boolean;
  placeholder?: string;
  invalid?: boolean;
  autoFocus?: boolean;
  onEnter?: () => void;
}) {
  return (
    <div className={`ac-input pw-input${mono ? ' is-mono' : ''}${invalid ? ' is-invalid' : ''}`}>
      {icon}
      {prefix && <span className="pw-input__prefix">{prefix}</span>}
      <input
        id={id}
        type="text"
        value={value}
        placeholder={placeholder}
        autoFocus={autoFocus}
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
    </div>
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

export function CountryFlag({ code }: { code: string }) {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    loadCountryFlag(code)
      .then((loaded) => {
        if (alive) setUrl(loaded ?? null);
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [code]);
  return url ? <img className="pw-flag" src={url} alt="" /> : <Globe size={15} className="ac-input__icon" aria-hidden="true" />;
}
