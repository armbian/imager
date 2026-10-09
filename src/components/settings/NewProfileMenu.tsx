// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useId, useRef, useState, type RefObject } from 'react';
import { createPortal } from 'react-dom';
import { ChevronDown, FileText, Plus, Wand2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { UI } from '../../config';
import { useAnchoredPopover } from '../../hooks/useAnchoredPopover';
import { useListboxKeys } from '../../hooks/useListboxKeys';

interface NewProfileMenuProps {
  onForm: () => void;
  onGuided: () => void;
}

interface NewProfileButtonProps extends NewProfileMenuProps {
  /** The trigger, so a dialog opened from the menu can give focus back to it */
  triggerRef: RefObject<HTMLButtonElement | null>;
}

export function NewProfileMenu({ onForm, onGuided, triggerRef: anchorRef }: NewProfileButtonProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  return (
    <>
      <button
        ref={anchorRef}
        type="button"
        className="btn btn-primary btn-pill"
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        <Plus size={16} aria-hidden="true" />
        {t('settings.autoconfig.newProfile')}
        <ChevronDown size={16} aria-hidden="true" className={`new-profile-chevron${open ? ' is-open' : ''}`} />
      </button>
      {open && (
        <NewProfileList
          anchorRef={anchorRef}
          onForm={() => {
            setOpen(false);
            onForm();
          }}
          onGuided={() => {
            setOpen(false);
            onGuided();
          }}
          onClose={() => setOpen(false)}
        />
      )}
    </>
  );
}

interface NewProfileListProps extends NewProfileMenuProps {
  anchorRef: React.RefObject<HTMLButtonElement | null>;
  onClose: () => void;
}

function NewProfileList({ anchorRef, onForm, onGuided, onClose }: NewProfileListProps) {
  const { t } = useTranslation();
  const listId = useId();
  const listRef = useRef<HTMLDivElement>(null);
  const items = [
    { Icon: FileText, label: t('settings.autoconfig.fillForm'), hint: t('settings.autoconfig.fillFormHint'), run: onForm },
    { Icon: Wand2, label: t('settings.autoconfig.guided'), hint: t('settings.autoconfig.guidedHint'), run: onGuided },
  ];

  const close = () => {
    onClose();
    anchorRef.current?.focus();
  };
  const keys = useListboxKeys(items.length, (index) => items[index].run(), close);
  const { popRef, style, placement } = useAnchoredPopover<HTMLDivElement>(anchorRef, true, {
    width: UI.NEW_PROFILE_MENU.WIDTH,
    prefer: 'down',
    align: 'start',
    gap: UI.PROFILE_MENU.GAP,
    onClose,
  });

  useEffect(() => {
    listRef.current?.focus({ preventScroll: true });
  }, []);

  return createPortal(
    <div ref={popRef} className={`profile-menu new-profile-menu is-${placement}`} style={style}>
      <div
        ref={listRef}
        className="profile-menu__list"
        role="listbox"
        tabIndex={0}
        aria-label={t('settings.autoconfig.newProfile')}
        aria-activedescendant={keys.active >= 0 ? `${listId}-${keys.active}` : undefined}
        onKeyDown={keys.onKeyDown}
      >
        {items.map(({ Icon, label, hint, run }, index) => (
          <div
            key={label}
            id={`${listId}-${index}`}
            role="option"
            aria-selected={false}
            className={`profile-menu__item new-profile-menu__item${index === keys.active ? ' is-active' : ''}`}
            onPointerMove={() => keys.setActive(index)}
            onClick={run}
          >
            <span className="profile-menu__newtile" aria-hidden="true">
              <Icon size={16} />
            </span>
            <span className="new-profile-menu__text">
              <span className="new-profile-menu__label">{label}</span>
              <span className="new-profile-menu__hint">{hint}</span>
            </span>
          </div>
        ))}
      </div>
    </div>,
    document.body
  );
}
