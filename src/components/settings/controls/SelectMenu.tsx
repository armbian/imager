// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useId, useMemo, useRef, useState, type CSSProperties, type KeyboardEvent, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { Check, ChevronDown, Search } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { UI } from '../../../config';
import { useAnchoredPopover } from '../../../hooks/useAnchoredPopover';
import { useListboxKeys } from '../../../hooks/useListboxKeys';
import { CodeBadge } from './CodeBadge';

export interface SelectOption<T extends string> {
  value: T;
  label: string;
  flagUrl?: string;
  /** Leading glyph for an option without a flag (the "System" language row). */
  icon?: ReactNode;
  badge?: string;
  hint?: string;
}

interface SelectMenuProps<T extends string> {
  value: T;
  options: readonly SelectOption<T>[];
  onChange: (value: T) => void;
  ariaLabel: string;
  searchable?: boolean;
  searchPlaceholder?: string;
  /** Trigger width in px; the menu opens at least UI.SELECT_MENU.MIN_WIDTH wide. */
  width?: number;
  disabled?: boolean;
}

function Lead({ option }: { option: SelectOption<string> }) {
  if (option.flagUrl) return <img className="set-select__flag" src={option.flagUrl} alt="" aria-hidden="true" />;
  if (option.icon) return <span className="set-select__flag">{option.icon}</span>;
  return null;
}

// Scrolls only the list, never the page behind the fixed menu.
function revealInList(list: HTMLElement, item: HTMLElement, center: boolean) {
  const top = item.offsetTop;
  const bottom = top + item.offsetHeight;
  if (center) {
    list.scrollTop = top - (list.clientHeight - item.offsetHeight) / 2;
  } else if (top < list.scrollTop) {
    list.scrollTop = top;
  } else if (bottom > list.scrollTop + list.clientHeight) {
    list.scrollTop = bottom - list.clientHeight;
  }
}

export function SelectMenu<T extends string>({
  value,
  options,
  onChange,
  ariaLabel,
  searchable = false,
  searchPlaceholder,
  width,
  disabled = false,
}: SelectMenuProps<T>) {
  const { t } = useTranslation();
  const listId = useId();
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const centerNext = useRef(false);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const needle = query.trim().toLowerCase();
  const shown = useMemo(() => {
    if (!needle) return options;
    const hits = options.filter((o) => `${o.label} ${o.hint ?? ''} ${o.value}`.toLowerCase().includes(needle));
    const rank = (o: SelectOption<T>) => (o.label.toLowerCase().startsWith(needle) ? 0 : 1);
    return [...hits].sort((a, b) => rank(a) - rank(b));
  }, [options, needle]);
  const selected = options.find((o) => o.value === value) ?? null;
  const hasLead = options.some((o) => o.flagUrl || o.icon);

  const close = () => {
    setOpen(false);
    triggerRef.current?.focus();
  };
  const pick = (index: number) => {
    const option = shown[index];
    if (!option) return;
    if (option.value !== value) onChange(option.value);
    close();
  };
  const keys = useListboxKeys(shown.length, pick, close);
  const { popRef, style } = useAnchoredPopover<HTMLDivElement>(triggerRef, open, {
    width: Math.max(width ?? 0, UI.SELECT_MENU.MIN_WIDTH),
    prefer: 'down',
    align: 'end',
    gap: UI.SELECT_MENU.GAP,
    onClose: () => setOpen(false),
  });

  useEffect(() => {
    const list = listRef.current;
    if (!open || !list || keys.active < 0) return;
    const item = document.getElementById(`${listId}-${keys.active}`);
    if (item) revealInList(list, item, centerNext.current);
    centerNext.current = false;
  }, [open, keys.active, listId]);

  useEffect(() => {
    if (open && !searchable) listRef.current?.focus();
  }, [open, searchable]);

  const openMenu = () => {
    setQuery('');
    keys.setActive(Math.max(0, options.findIndex((o) => o.value === value)));
    centerNext.current = true;
    setOpen(true);
  };
  const onTriggerKey = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (!open && (event.key === 'ArrowDown' || event.key === 'ArrowUp')) {
      event.preventDefault();
      openMenu();
    }
  };
  const activeId = keys.active >= 0 ? `${listId}-${keys.active}` : undefined;

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        className={`set-select${open ? ' is-open' : ''}`}
        style={width ? ({ '--select-w': `${width}px` } as CSSProperties) : undefined}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={ariaLabel}
        disabled={disabled}
        onClick={() => (open ? setOpen(false) : openMenu())}
        onKeyDown={onTriggerKey}
      >
        {selected && <Lead option={selected} />}
        <span className="set-select__value">{selected?.label}</span>
        {selected?.badge && <CodeBadge>{selected.badge}</CodeBadge>}
        <ChevronDown size={16} className="set-select__chevron" aria-hidden="true" />
      </button>
      {open &&
        createPortal(
          <div ref={popRef} className="set-menu" style={style}>
            {searchable && (
              <div className="set-menu__search">
                <Search size={14} aria-hidden="true" />
                <input
                  type="text"
                  value={query}
                  placeholder={searchPlaceholder}
                  aria-label={searchPlaceholder ?? ariaLabel}
                  aria-controls={listId}
                  aria-activedescendant={activeId}
                  autoFocus
                  spellCheck={false}
                  onChange={(e) => {
                    setQuery(e.target.value);
                    keys.setActive(0);
                  }}
                  onKeyDown={keys.onKeyDown}
                />
              </div>
            )}
            <div
              ref={listRef}
              id={listId}
              className="set-menu__list"
              role="listbox"
              aria-label={ariaLabel}
              aria-activedescendant={searchable ? undefined : activeId}
              tabIndex={searchable ? undefined : -1}
              onKeyDown={searchable ? undefined : keys.onKeyDown}
            >
              {shown.length === 0 ? (
                <div className="set-menu__empty">{t('settings.autoconfig.wizard.noMatch')}</div>
              ) : (
                shown.map((o, i) => (
                  <div
                    key={o.value}
                    id={`${listId}-${i}`}
                    role="option"
                    aria-selected={o.value === value}
                    className={`set-menu__opt${i === keys.active ? ' is-active' : ''}${o.value === value ? ' is-sel' : ''}${o.badge ? ' has-badge' : ''}`}
                    onPointerMove={() => keys.setActive(i)}
                    onClick={() => pick(i)}
                  >
                    <span className="set-menu__check">{o.value === value && <Check size={14} aria-hidden="true" />}</span>
                    {hasLead && (o.flagUrl || o.icon ? <Lead option={o} /> : <span className="set-select__flag" />)}
                    <span className="set-menu__label">{o.label}</span>
                    {o.hint && <span className="set-menu__hint">{o.hint}</span>}
                    {o.badge && <CodeBadge>{o.badge}</CodeBadge>}
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
