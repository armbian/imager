// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { forwardRef, useEffect, useId, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronRight, CircleCheck, RotateCcw, TriangleAlert } from 'lucide-react';
import { loadCountryFlag } from '../../../config/countryFlags';
import type { EditorGroup } from '../../../config/profileEditorModel';

interface EditorSectionProps {
  id: EditorGroup;
  eyebrow: string;
  description: string;
  attention: boolean;
  /** A small link at the end of the head, such as Use Armbian defaults */
  action?: ReactNode;
  children: ReactNode;
}

// SettingsGroup has no description line under the eyebrow, which every editor group carries.
export function EditorSection({ id, eyebrow, description, attention, action, children }: EditorSectionProps) {
  const headingId = useId();
  return (
    <section className="pe-group set-group" data-group={id} aria-labelledby={headingId}>
      <div className="pe-group__head">
        <div className="pe-group__titles">
          <h3 id={headingId} className="set-group__eyebrow pe-group__eyebrow">
            {attention && <span className="set-group__dot" aria-hidden="true" />}
            {eyebrow}
          </h3>
          <p className="pe-group__desc">{description}</p>
        </div>
        {action}
      </div>
      <div className="set-group__card">{children}</div>
    </section>
  );
}

/** Shown on an applied area: it stops writing it, so Armbian keeps its own defaults there */
export function DefaultsAction({ onClick }: { onClick: (byPointer: boolean) => void }) {
  const { t } = useTranslation();
  return (
    <button type="button" className="pe-group__reset" onClick={(event) => onClick(event.detail > 0)}>
      <RotateCcw size={12} aria-hidden="true" />
      {t('settings.autoconfig.editor.useDefaults')}
    </button>
  );
}

type HintTone = 'ok' | 'warn' | 'plain';

export function FieldHint({ tone = 'plain', children }: { tone?: HintTone; children: ReactNode }) {
  return (
    <span className={`pe-hint is-${tone}`}>
      {tone === 'ok' && <CircleCheck size={12} aria-hidden="true" />}
      {tone === 'warn' && <TriangleAlert size={12} aria-hidden="true" />}
      <span className="pe-hint__text">{children}</span>
    </span>
  );
}

interface TextFieldProps {
  id: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  icon?: ReactNode;
  mono?: boolean;
  invalid?: boolean;
  describedBy?: string;
}

export const TextField = forwardRef<HTMLInputElement, TextFieldProps>(function TextField(
  { id, value, onChange, placeholder, icon, mono = false, invalid = false, describedBy },
  ref
) {
  return (
    <span className={`pe-input${mono ? ' is-mono' : ''}${invalid ? ' is-invalid' : ''}`}>
      {icon}
      <input
        ref={ref}
        id={id}
        type="text"
        value={value}
        placeholder={placeholder}
        aria-invalid={invalid || undefined}
        aria-describedby={describedBy}
        autoComplete="off"
        autoCapitalize="off"
        autoCorrect="off"
        spellCheck={false}
        onChange={(e) => onChange(e.target.value)}
      />
    </span>
  );
});

interface MoreRowProps {
  label: string;
  value: string;
  open: boolean;
  controls: string;
  onToggle: () => void;
}

export function MoreRow({ label, value, open, controls, onToggle }: MoreRowProps) {
  return (
    <button
      type="button"
      className={`set-row pe-more${open ? ' is-open' : ''}`}
      aria-expanded={open}
      aria-controls={controls}
      onClick={onToggle}
    >
      <span className="set-row__text">
        <span className="set-row__label">{label}</span>
      </span>
      <span className="set-row__control">
        <span className="pe-more__value">{value}</span>
        <ChevronRight size={16} className="pe-more__chev" aria-hidden="true" />
      </span>
    </button>
  );
}

/** Twemoji flag of a country code, or the code itself in a chip when Twemoji has none. */
export function CountryFlag({ code }: { code: string }) {
  const [flag, setFlag] = useState<{ code: string; url: string | null } | null>(null);
  useEffect(() => {
    let alive = true;
    loadCountryFlag(code)
      .then((url) => {
        if (alive) setFlag({ code, url: url ?? null });
      })
      .catch(() => {
        if (alive) setFlag({ code, url: null });
      });
    return () => {
      alive = false;
    };
  }, [code]);
  if (!flag || flag.code !== code) return <span className="pe-flag" aria-hidden="true" />;
  return flag.url ? (
    <img className="pe-flag" src={flag.url} alt="" aria-hidden="true" />
  ) : (
    <span className="pe-flag pe-flag--code" aria-hidden="true">{code}</span>
  );
}
