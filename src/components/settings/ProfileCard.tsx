// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronRight, Cpu, Layers } from 'lucide-react';
import { UI } from '../../config';
import { profileBoards } from '../../config/autoconfig';
import { profileScopeLabel, profileSpec } from '../../config/profileEditorModel';
import type { AutoconfigProfile } from '../../types';
import { staggerDelay } from '../../utils';
import { FactGlyph } from './editor/FactGlyph';
import { ProfileTile } from './ProfileTile';

interface ProfileCardProps {
  profile: AutoconfigProfile;
  onEdit: (profile: AutoconfigProfile) => void;
  /** Its place on the page, for the staggered entrance */
  index?: number;
}

// A value cut by its ellipsis gets the full text as a tooltip; one that fits gets none
function titleIfCut(event: React.MouseEvent<HTMLElement>, text: string) {
  const el = event.currentTarget;
  el.title = el.scrollWidth > el.clientWidth + 1 ? text : '';
}

/** A profile in the list: the board tile, name over scope, then the four first-boot areas */
export function ProfileCard({ profile, onEdit, index = 0 }: ProfileCardProps) {
  const { t } = useTranslation();
  const boards = profileBoards(profile);
  const spec = profileSpec(profile, t);
  const ref = useRef<HTMLElement>(null);
  const [wide, setWide] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const observer = new ResizeObserver(() => setWide(el.clientWidth >= UI.PROFILES.WIDE_CARD_MIN));
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return (
    <article
      ref={ref}
      className={`profile-card mfr-card--enter${wide ? ' is-wide' : ''}`}
      style={{ animationDelay: staggerDelay(index) }}
      onClick={() => onEdit(profile)}
    >
      <ProfileTile boards={boards} />
      <div className="profile-card__body">
        <div className="profile-card__head">
          {/* Its click bubbles to the card, which opens the editor from anywhere */}
          <button type="button" className="profile-card__open">
            {profile.name}
          </button>
          <span className="profile-card__scope">
            {boards.length ? <Cpu size={13} aria-hidden="true" /> : <Layers size={13} aria-hidden="true" />}
            <span>{profileScopeLabel(boards, t)}</span>
          </span>
        </div>
        <dl className="profile-card__spec">
          {spec.map((fact) => (
            <div key={fact.label} className={`profile-card__fact is-${fact.tone}`} title={wide ? undefined : t(fact.label)}>
              <dt>
                <span className="profile-card__glyph">
                  <FactGlyph icon={fact.icon} size={wide ? 12 : 14} />
                </span>
                <span className="profile-card__label">{t(fact.label)}</span>
              </dt>
              <dd onMouseEnter={(e) => titleIfCut(e, fact.text)}>{fact.text}</dd>
            </div>
          ))}
        </dl>
      </div>
      <span className="profile-card__chevron" aria-hidden="true">
        <ChevronRight size={16} />
      </span>
    </article>
  );
}
