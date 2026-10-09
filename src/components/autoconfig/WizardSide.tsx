// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { Trans, useTranslation } from 'react-i18next';
import { Cable, Globe, KeyRound, User, Wifi } from 'lucide-react';
import { UI } from '../../config';
import { profileScopeLabel } from '../../config/profileEditorModel';
import { WIZARD_DEFAULT_KEY, WIZARD_ITEMS, WIZARD_STEPS, wizardItemFact, type WizardItem } from '../../config/profileWizard';
import type { ProfileWizardState } from '../../hooks/useProfileWizard';
import { BoardImage } from '../shared/BoardImage';

const ITEM_ICON = { network: Wifi, user: User, locale: Globe, access: KeyRound } as const;

/** The glyph of a wizard item, shared by the side column and the review */
export function WizardItemIcon({ item, cable }: { item: WizardItem; cable: boolean }) {
  const Icon = item === 'network' && cable ? Cable : ITEM_ICON[item];
  return <Icon size={UI.ICON_SIZE.WIZARD_ITEM} />;
}

interface WizardSideProps {
  wizard: ProfileWizardState;
  boardImage: string | null;
}

export function WizardSide({ wizard, boardImage }: WizardSideProps) {
  const { t } = useTranslation();
  const { model, skipped, step, stepIndex, furthest, facts } = wizard;
  const name = model.name.trim();

  return (
    <aside className="pw-side">
      {boardImage && (
        <div className="pw-side__board">
          <BoardImage src={boardImage} alt="" />
        </div>
      )}
      <span className="pw-side__eyebrow">{t('settings.autoconfig.wizard.eyebrow')}</span>
      <div className={`pw-side__name${name ? '' : ' is-empty'}`}>
        {name || t('settings.autoconfig.wizard.unnamed')}
      </div>
      <div className="pw-side__for">{model.scope === 'some' && model.boards.length === 0
          ? t('settings.autoconfig.editor.noBoardPicked')
          : profileScopeLabel(model.scope === 'some' ? model.boards : [], t)}</div>
      <ol className="pw-items">
        {WIZARD_ITEMS.map((item) => {
          const at = WIZARD_STEPS.indexOf(item);
          // Steps already passed keep their summary when Edit on the review jumps back.
          const fact = wizardItemFact(item, facts);
          // A set step Armbian will not apply (the region without a user) reads like a skipped one.
          const passed = skipped[item] || fact.tone === 'off' ? 'skip' : 'done';
          const state = stepIndex === at ? 'cur' : furthest > at ? passed : '';
          return (
            <li key={item} className={`pw-item${state ? ` is-${state}` : ''}`} aria-current={state === 'cur' ? 'step' : undefined}>
              <span className="pw-item__icon" aria-hidden="true">
                <WizardItemIcon item={item} cable={model.network.apply && model.network.mode === 'cable'} />
              </span>
              <span className="pw-item__text">
                {t(`settings.autoconfig.wizard.steps.${item}`)}
                {state === 'done' || state === 'skip' ? (
                  <small className={`is-${fact.tone}`}>{fact.text}</small>
                ) : (
                  <small>{t(WIZARD_DEFAULT_KEY[item])}</small>
                )}
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
