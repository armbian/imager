// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { Cable, Globe, KeyRound, User, Wifi } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { UI } from '../../config';
import { profileGlyphs, type ProfileGlyph } from '../../config/autoconfig';
import type { AutoconfigConfig } from '../../types';

const GLYPH_ICON = { wifi: Wifi, cable: Cable, user: User, timezone: Globe, keys: KeyRound } as const satisfies Record<
  ProfileGlyph,
  typeof Wifi
>;

export function ProfileGlyphs({ config }: { config: AutoconfigConfig }) {
  const { t, i18n } = useTranslation();
  const glyphs = profileGlyphs(config);
  if (glyphs.length === 0) return null;
  const names = glyphs.map((g) => t(`flash.profile.glyph.${g}`));
  const list = new Intl.ListFormat(i18n.language, { type: 'unit' }).format(names);
  return (
    <span className="profile-glyphs" role="img" aria-label={t('flash.profile.glyphsLabel', { list })} title={list}>
      {glyphs.map((g) => {
        const Icon = GLYPH_ICON[g];
        return (
          <i key={g} aria-hidden="true">
            <Icon size={UI.ICON_SIZE.PROFILE_GLYPH} />
          </i>
        );
      })}
    </span>
  );
}
