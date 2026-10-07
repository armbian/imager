// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useId, useState } from 'react';
import type { LucideIcon } from 'lucide-react';
import { DEV_SCENARIOS, type DevOption } from '../../config/devScenarios';

const { ICON_SIZE } = DEV_SCENARIOS;

interface DevChoiceProps<T extends string | number> {
  label: string;
  options: DevOption<T>[];
  value: T;
  disabled?: boolean;
  onChange: (value: T) => void;
}

export function DevSegButtons<T extends string | number>({ label, options, value, disabled, onChange }: DevChoiceProps<T>) {
  return (
    <div className="dev-seg" role="group" aria-label={label}>
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          className="dev-seg__btn"
          aria-pressed={o.value === value}
          disabled={disabled}
          onClick={() => o.value !== value && onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function DevSegmented<T extends string>(props: DevChoiceProps<T>) {
  return (
    <div className="dev-field">
      <span className="dev-field__label">{props.label}</span>
      <DevSegButtons {...props} />
    </div>
  );
}

export function DevRadioList<T extends string>({ label, options, value, disabled, onChange }: DevChoiceProps<T>) {
  const labelId = useId();
  const name = useId();
  return (
    <div className="dev-field">
      <span id={labelId} className="dev-field__label">
        {label}
      </span>
      <div className="dev-radios" role="radiogroup" aria-labelledby={labelId}>
        {options.map((o) => (
          <label key={o.value} className={`dev-radio${o.value === value ? ' is-checked' : ''}${disabled ? ' is-disabled' : ''}`}>
            <input
              type="radio"
              name={name}
              value={o.value}
              checked={o.value === value}
              disabled={disabled}
              onChange={() => onChange(o.value)}
            />
            <span className="dev-radio__mark" aria-hidden="true" />
            <span className="dev-radio__label">{o.label}</span>
          </label>
        ))}
      </div>
    </div>
  );
}

interface DevNumberFieldProps {
  label: string;
  unit: string;
  value: number;
  min: number;
  max: number;
  disabled?: boolean;
  onCommit: (value: number) => void;
}

/** Commits on blur or Enter, not per keystroke: every scenario write restarts the hot-plug clock. */
export function DevNumberField({ label, unit, value, min, max, disabled, onCommit }: DevNumberFieldProps) {
  const inputId = useId();
  const [draft, setDraft] = useState<string | null>(null);

  const commit = () => {
    if (draft === null) return;
    const parsed = Math.round(Number(draft));
    setDraft(null);
    if (!Number.isFinite(parsed) || draft.trim() === '') return;
    const clamped = Math.min(max, Math.max(min, parsed));
    if (clamped !== value) onCommit(clamped);
  };

  return (
    <div className="dev-field dev-field--number">
      <label className="dev-field__label" htmlFor={inputId}>
        {label}
      </label>
      <div className={`dev-input dev-input--unit${disabled ? ' is-disabled' : ''}`}>
        <input
          id={inputId}
          type="number"
          inputMode="numeric"
          min={min}
          max={max}
          value={draft ?? String(value)}
          disabled={disabled}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === 'Enter') commit();
            if (e.key === 'Escape' && draft !== null) {
              e.stopPropagation();
              setDraft(null);
            }
          }}
        />
        <span className="dev-input__unit">{unit}</span>
      </div>
    </div>
  );
}

interface DevSwitchProps {
  label: string;
  hint?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}

export function DevSwitch({ label, hint, checked, disabled, onChange }: DevSwitchProps) {
  return (
    <div className="dev-switch">
      <div className="dev-switch__text">
        <span className="dev-switch__label">{label}</span>
        {hint && <span className="dev-switch__hint">{hint}</span>}
      </div>
      <label className="toggle-switch">
        <input
          type="checkbox"
          checked={checked}
          disabled={disabled}
          onChange={(e) => onChange(e.target.checked)}
          aria-label={label}
        />
        <span className="toggle-slider" />
      </label>
    </div>
  );
}

interface DevEmptyProps {
  icon: LucideIcon;
  title: string;
  hint: string;
}

export function DevEmpty({ icon: Icon, title, hint }: DevEmptyProps) {
  return (
    <div className="dev-empty">
      <Icon size={ICON_SIZE.LG} className="dev-empty__icon" aria-hidden="true" />
      <div>
        <p className="dev-empty__title">{title}</p>
        <p className="dev-empty__hint">{hint}</p>
      </div>
    </div>
  );
}
