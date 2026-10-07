// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useId, useRef, type RefObject } from 'react';
import { createPortal } from 'react-dom';
import { Check, Plus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { UI } from '../../config';
import { useAnchoredPopover } from '../../hooks/useAnchoredPopover';
import { useListboxKeys } from '../../hooks/useListboxKeys';
import type { AutoconfigProfile } from '../../types';
import { ProfileGlyphs } from './ProfileGlyphs';

interface ProfileMenuProps {
  anchorRef: RefObject<HTMLButtonElement | null>;
  profiles: readonly AutoconfigProfile[];
  selectedId: string;
  onSelect: (id: string) => void;
  onNewProfile: () => void;
  onClose: () => void;
}

export function ProfileMenu({ anchorRef, profiles, selectedId, onSelect, onNewProfile, onClose }: ProfileMenuProps) {
  const { t } = useTranslation();
  const listId = useId();
  const listRef = useRef<HTMLDivElement>(null);
  const ids = ['', ...profiles.map((p) => p.id)];
  const newIndex = ids.length;

  const close = () => {
    onClose();
    anchorRef.current?.focus();
  };
  const pick = (index: number) => {
    if (index === newIndex) onNewProfile();
    else onSelect(ids[index]);
  };
  const keys = useListboxKeys(ids.length + 1, pick, close);
  const { popRef, style, placement } = useAnchoredPopover<HTMLDivElement>(anchorRef, true, {
    width: UI.PROFILE_MENU.WIDTH,
    prefer: 'up',
    align: 'end',
    gap: UI.PROFILE_MENU.GAP,
    onClose,
  });

  const { setActive } = keys;
  useEffect(() => {
    setActive(Math.max(0, ids.indexOf(selectedId)));
    listRef.current?.focus({ preventScroll: true });
    // eslint-disable-next-line react-hooks/exhaustive-deps -- only on open
  }, []);

  const optionId = (index: number) => `${listId}-${index}`;
  const rowClass = (index: number, extra = '') =>
    `profile-menu__item${extra}${index === keys.active ? ' is-active' : ''}`;

  return createPortal(
    <div ref={popRef} className={`profile-menu is-${placement}`} style={style}>
      <div
        ref={listRef}
        className="profile-menu__list"
        role="listbox"
        tabIndex={0}
        aria-label={t('flash.profile.rowLabel')}
        aria-activedescendant={keys.active >= 0 ? optionId(keys.active) : undefined}
        onKeyDown={keys.onKeyDown}
      >
        {ids.map((id, index) => {
          const profile = profiles.find((p) => p.id === id) ?? null;
          const isSelected = id === selectedId;
          const label = profile ? profile.name : t('flash.profile.none');
          return (
            <div
              key={id || 'none'}
              id={optionId(index)}
              role="option"
              aria-selected={isSelected}
              className={rowClass(index, `${isSelected ? ' is-sel' : ''}${profile ? '' : ' is-none'}`)}
              onPointerMove={() => keys.setActive(index)}
              onClick={() => pick(index)}
            >
              <span className="profile-menu__name">{label}</span>
              {profile && <ProfileGlyphs config={profile.config} />}
              <span className="profile-menu__check" aria-hidden="true">
                {isSelected && <Check size={16} />}
              </span>
            </div>
          );
        })}
        <div className="profile-menu__sep" role="presentation" />
        <div
          id={optionId(newIndex)}
          role="option"
          aria-selected={false}
          className={rowClass(newIndex, ' is-new')}
          onPointerMove={() => keys.setActive(newIndex)}
          onClick={() => pick(newIndex)}
        >
          <span className="profile-menu__newtile" aria-hidden="true">
            <Plus size={14} />
          </span>
          <span className="profile-menu__name">{t('flash.profile.newProfile')}</span>
        </div>
      </div>
    </div>,
    document.body
  );
}
