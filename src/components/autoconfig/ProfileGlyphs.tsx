// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { Cable, Globe, KeyRound, User, Wifi } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { UI } from '../../config';
import { presetIsEmpty, profileGlyphs, type ProfileGlyph } from '../../config/autoconfig';
import type { AutoconfigConfig } from '../../types';
import { formatList } from '../../utils';

const GLYPH_ICON = { wifi: Wifi, cable: Cable, user: User, timezone: Globe, keys: KeyRound } as const satisfies Record<
  ProfileGlyph,
  typeof Wifi
>;

export function ProfileGlyphIcon({ glyph, size = UI.ICON_SIZE.PROFILE_GLYPH }: { glyph: ProfileGlyph; size?: number }) {
  const Icon = GLYPH_ICON[glyph];
  return <Icon size={size} aria-hidden="true" />;
}

export function ProfileGlyphs({ config }: { config: AutoconfigConfig }) {
  const { t, i18n } = useTranslation();
  if (presetIsEmpty(config)) return <span className="profile-glyphs is-empty">{t('flash.profile.noChanges')}</span>;
  const glyphs = profileGlyphs(config);
  if (glyphs.length === 0) return null;
  const names = glyphs.map((g) => t(`flash.profile.glyph.${g}`));
  const list = formatList(names, i18n.language);
  return (
    <span className="profile-glyphs" role="img" aria-label={t('flash.profile.glyphsLabel', { list })} title={list}>
      {glyphs.map((g) => (
        <i key={g} aria-hidden="true">
          <ProfileGlyphIcon glyph={g} />
        </i>
      ))}
    </span>
  );
}
