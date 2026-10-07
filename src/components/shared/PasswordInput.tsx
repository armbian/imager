// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useState, type ReactNode } from 'react';
import { Eye, EyeOff, Lock } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface PasswordInputProps {
  value: string | undefined;
  onChange: (value: string) => void;
  placeholder?: string;
  id?: string;
  trailing?: ReactNode;
}

/** Password field: never autofilled, autocorrected or sent to a cloud spellchecker. */
export function PasswordInput({ value, onChange, placeholder, id, trailing }: PasswordInputProps) {
  const { t } = useTranslation();
  const [show, setShow] = useState(false);
  return (
    <div className="ac-input">
      <Lock size={15} className="ac-input__icon" aria-hidden="true" />
      <input
        id={id}
        type={show ? 'text' : 'password'}
        value={value ?? ''}
        placeholder={placeholder}
        autoComplete="new-password"
        autoCapitalize="off"
        autoCorrect="off"
        spellCheck={false}
        onChange={(e) => onChange(e.target.value)}
      />
      {trailing}
      <button
        type="button"
        className="ac-input__btn"
        onClick={() => setShow((s) => !s)}
        aria-label={show ? t('common.hidePassword') : t('common.showPassword')}
        aria-pressed={show}
      >
        {show ? <EyeOff size={15} /> : <Eye size={15} />}
      </button>
    </div>
  );
}
