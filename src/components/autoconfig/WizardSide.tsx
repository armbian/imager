// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { Trans, useTranslation } from 'react-i18next';
import { Globe, KeyRound, User, Wifi } from 'lucide-react';
import { UI } from '../../config';
import { WIZARD_ITEMS, WIZARD_STEPS, WIZARD_SUMMARY_SEPARATOR, wizardItemSummary } from '../../config/profileWizard';
import type { ProfileWizardState } from '../../hooks/useProfileWizard';
import { BoardImage } from '../shared/BoardImage';

const ITEM_ICON = { network: Wifi, user: User, locale: Globe, access: KeyRound } as const;

interface WizardSideProps {
  wizard: ProfileWizardState;
  boardName: string | null;
  boardImage: string | null;
}

export function WizardSide({ wizard, boardName, boardImage }: WizardSideProps) {
  const { t } = useTranslation();
  const { draft, step, stepIndex } = wizard;
  const name = draft.name.trim();

  return (
    <aside className="pw-side">
      <div className="pw-side__board">
        <BoardImage src={boardImage ?? undefined} alt="" />
      </div>
      <span className="pw-side__eyebrow">{t('settings.autoconfig.wizard.eyebrow')}</span>
      <div className={`pw-side__name${name ? '' : ' is-empty'}`}>
        {name || t('settings.autoconfig.wizard.unnamed')}
      </div>
      <div className="pw-side__for">
        {boardName ? t('settings.autoconfig.wizard.forBoard', { board: boardName }) : t('settings.autoconfig.wizard.forAnyBoard')}
      </div>
      <ol className="pw-items">
        {WIZARD_ITEMS.map((item) => {
          const at = WIZARD_STEPS.indexOf(item);
          const state = stepIndex === at ? 'cur' : stepIndex > at ? (draft.skipped[item] ? 'skip' : 'done') : '';
          const Icon = ITEM_ICON[item];
          const summary = state === 'done' ? wizardItemSummary(item, draft, t) : [];
          return (
            <li key={item} className={`pw-item${state ? ` is-${state}` : ''}`} aria-current={state === 'cur' ? 'step' : undefined}>
              <span className="pw-item__icon" aria-hidden="true">
                <Icon size={UI.ICON_SIZE.WIZARD_ITEM} />
              </span>
              <span className="pw-item__text">
                {t(`settings.autoconfig.wizard.steps.${item}`)}
                <small>
                  {summary.length > 0 ? summary.join(WIZARD_SUMMARY_SEPARATOR.SIDE) : t(`settings.autoconfig.wizard.pending.${item}`)}
                </small>
              </span>
              <i className={`pw-dot${state ? ` is-${state}` : ''}`} aria-hidden="true" />
            </li>
          );
        })}
      </ol>
      <div className="pw-side__grow" />
      <div className="pw-progress">
        <div className="pw-progress__label">
          <span>
            <Trans i18nKey="settings.autoconfig.wizard.stepOf" values={{ current: stepIndex + 1, total: WIZARD_STEPS.length }} components={{ b: <b /> }} />
          </span>
          <span>{t(`settings.autoconfig.wizard.steps.${step}`)}</span>
        </div>
        <div className="pw-progress__bar" aria-hidden="true">
          {WIZARD_STEPS.map((s, i) => (
            <i key={s} className={i < stepIndex ? 'is-done' : i === stepIndex ? 'is-cur' : ''} />
          ))}
        </div>
      </div>
    </aside>
  );
}
