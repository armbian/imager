// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useId, useMemo, useRef, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { Check, ChevronDown, Search } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { UI } from '../../config';
import { useAnchoredPopover } from '../../hooks/useAnchoredPopover';
import { useListboxKeys } from '../../hooks/useListboxKeys';

export interface PickerOption {
  value: string;
  label: string;
  meta?: string;
  icon?: ReactNode;
}

interface SearchPickerProps {
  value: string;
  options: readonly PickerOption[];
  onChange: (value: string) => void;
  label: string;
  searchPlaceholder: string;
  placeholder: string;
  icon?: ReactNode;
}

function Highlight({ text, query }: { text: string; query: string }) {
  if (!query) return <>{text}</>;
  const lower = text.toLowerCase();
  const parts: ReactNode[] = [];
  let from = 0;
  for (let at = lower.indexOf(query, from); at !== -1; at = lower.indexOf(query, from)) {
    if (at > from) parts.push(text.slice(from, at));
    parts.push(<mark key={at}>{text.slice(at, at + query.length)}</mark>);
    from = at + query.length;
  }
  parts.push(text.slice(from));
  return <>{parts}</>;
}

export function SearchPicker({ value, options, onChange, label, searchPlaceholder, placeholder, icon }: SearchPickerProps) {
  const { t } = useTranslation();
  const listId = useId();
  const triggerRef = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const needle = query.trim().toLowerCase();
  // Matches at the start of the label come first: "it" finds Italy before United Kingdom.
  const shown = useMemo(() => {
    if (!needle) return options;
    const hits = options.filter((o) => `${o.label} ${o.value}`.toLowerCase().includes(needle));
    const rank = (o: PickerOption) => (o.label.toLowerCase().startsWith(needle) ? 0 : 1);
    return [...hits].sort((a, b) => rank(a) - rank(b));
  }, [options, needle]);
  const selected = options.find((o) => o.value === value) ?? null;

  const close = () => {
    setOpen(false);
    triggerRef.current?.focus();
  };
  const pick = (index: number) => {
    const option = shown[index];
    if (!option) return;
    onChange(option.value);
    close();
  };
  const keys = useListboxKeys(shown.length, pick, close);
  const { popRef, style } = useAnchoredPopover<HTMLDivElement>(triggerRef, open, {
    width: null,
    prefer: 'down',
    align: 'start',
    gap: UI.PICKER.GAP,
    onClose: () => setOpen(false),
  });

  useEffect(() => {
    if (open && keys.active >= 0) document.getElementById(`${listId}-${keys.active}`)?.scrollIntoView({ block: 'nearest' });
  }, [open, keys.active, listId]);

  const toggle = () => {
    if (open) {
      setOpen(false);
      return;
    }
    setQuery('');
    keys.setActive(Math.max(0, options.findIndex((o) => o.value === value)));
    setOpen(true);
  };

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        className={`ac-input is-select search-picker__trigger${open ? ' is-open' : ''}`}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={label}
        onClick={toggle}
      >
        {selected?.icon ?? icon}
        <span className={`search-picker__value${selected ? '' : ' is-empty'}`}>{selected?.label ?? placeholder}</span>
        <ChevronDown size={15} className="ac-input__chevron" aria-hidden="true" />
      </button>
      {open &&
        createPortal(
          <div ref={popRef} className="search-picker" style={style}>
            <div className="search-picker__search">
              <Search size={14} aria-hidden="true" />
              <input
                type="text"
                value={query}
                placeholder={searchPlaceholder}
                aria-label={searchPlaceholder}
                aria-controls={listId}
                aria-activedescendant={keys.active >= 0 ? `${listId}-${keys.active}` : undefined}
                autoFocus
                spellCheck={false}
                onChange={(e) => {
                  setQuery(e.target.value);
                  keys.setActive(0);
                }}
                onKeyDown={keys.onKeyDown}
              />
            </div>
            <div id={listId} className="search-picker__list" role="listbox" aria-label={label}>
              {shown.length === 0 ? (
                <div className="search-picker__empty">{t('settings.autoconfig.wizard.noMatch')}</div>
              ) : (
                shown.map((o, i) => (
                  <div
                    key={o.value}
                    id={`${listId}-${i}`}
                    role="option"
                    aria-selected={o.value === value}
                    className={`search-picker__item${i === keys.active ? ' is-active' : ''}${o.value === value ? ' is-sel' : ''}`}
                    onPointerMove={() => keys.setActive(i)}
                    onClick={() => pick(i)}
                  >
                    {o.icon}
                    <span className="search-picker__label">
                      <Highlight text={o.label} query={needle} />
                    </span>
                    {o.meta && <small className="search-picker__meta">{o.meta}</small>}
                    {o.value === value && <Check size={14} className="search-picker__check" aria-hidden="true" />}
                  </div>
                ))
              )}
            </div>
          </div>,
          document.body
        )}
    </>
  );
}
