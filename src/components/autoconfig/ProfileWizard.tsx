// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useId, type RefObject } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { ArrowRight, Cable, Check, Globe, KeyRound, Tag, User, Wifi, X } from 'lucide-react';
import { UI } from '../../config';
import { isWizardItem } from '../../config/profileWizard';
import { useDialogFocus } from '../../hooks/useDialogFocus';
import { useModalExitAnimation } from '../../hooks/useModalExitAnimation';
import { useProfileWizard, type ProfileWizardState } from '../../hooks/useProfileWizard';
import type { AutoconfigProfile } from '../../types';
import { WizardAccessStep } from './WizardAccessStep';
import { WizardField, WizardTextField } from './WizardFields';
import { WizardLocaleStep } from './WizardLocaleStep';
import { WizardNetworkStep } from './WizardNetworkStep';
import { WizardReviewStep } from './WizardReviewStep';
import { WizardSide } from './WizardSide';
import { WizardUserStep } from './WizardUserStep';

const STEP_ICON = { name: Tag, network: Wifi, user: User, locale: Globe, access: KeyRound, review: Check } as const;

interface ProfileWizardProps {
  boardName: string | null;
  boardImage: string | null;
  returnFocusRef: RefObject<HTMLElement | null>;
  onClose: () => void;
  onCreated: (profile: AutoconfigProfile) => void;
}

function WizardStepBody({ wizard, uid }: { wizard: ProfileWizardState; uid: string }) {
  const { t } = useTranslation();
  const { draft, attempted, stepError, setName, next, shownError } = wizard;
  switch (wizard.step) {
    case 'name':
      return (
        <WizardField
          label={t('settings.autoconfig.nameLabel')}
          htmlFor={`${uid}-name`}
          hint={t('settings.autoconfig.wizard.nameHint')}
          error={shownError(stepError)}
        >
          <WizardTextField
            id={`${uid}-name`}
            value={draft.name}
            autoFocus
            icon={<Tag size={15} className="ac-input__icon" aria-hidden="true" />}
            placeholder={t('settings.autoconfig.namePlaceholder')}
            invalid={attempted && !!stepError}
            onChange={setName}
            onEnter={next}
          />
        </WizardField>
      );
    case 'network':
      return <WizardNetworkStep wizard={wizard} uid={uid} />;
    case 'user':
      return <WizardUserStep wizard={wizard} uid={uid} />;
    case 'locale':
      return <WizardLocaleStep wizard={wizard} />;
    case 'access':
      return <WizardAccessStep wizard={wizard} uid={uid} />;
    case 'review':
      return <WizardReviewStep wizard={wizard} />;
  }
}

export function ProfileWizard({ boardName, boardImage, returnFocusRef, onClose, onCreated }: ProfileWizardProps) {
  const { t } = useTranslation();
  const uid = useId();
  const titleId = `${uid}-title`;
  const { isExiting, handleClose } = useModalExitAnimation({ onClose });
  const wizard = useProfileWizard((profile) => {
    onCreated(profile);
    handleClose();
  });
  const { draft, step, stepIndex, saving, blocked, attempted, go, next, skip, create } = wizard;
  const dialogRef = useDialogFocus<HTMLDivElement>({ onEscape: saving ? undefined : handleClose, returnFocusRef });

  const HeadIcon = step === 'network' && draft.network.mode === 'cable' ? Cable : STEP_ICON[step];
  const title = step === 'review'
    ? t('settings.autoconfig.wizard.review.title', { name: draft.name.trim() })
    : t(`settings.autoconfig.wizard.${step}.title`);
  const sub = step === 'review' && !boardName
    ? t('settings.autoconfig.wizard.review.subSettings')
    : t(`settings.autoconfig.wizard.${step}.sub`);

  return createPortal(
    <div className={`pw-overlay${isExiting ? ' is-exiting' : ''}`}>
      <div ref={dialogRef} className="pw-card" role="dialog" aria-modal="true" aria-labelledby={titleId}>
        <WizardSide wizard={wizard} boardName={boardName} boardImage={boardImage} />

        <div className="pw-main">
          <button type="button" className="pw-x" onClick={handleClose} aria-label={t('settings.autoconfig.wizard.close')}>
            <X size={17} />
          </button>
          <header className="pw-head">
            <span className="pw-head__tile" aria-hidden="true">
              <HeadIcon size={UI.ICON_SIZE.WIZARD_TILE} />
            </span>
            <div>
              <h2 id={titleId} className="pw-head__title">{title}</h2>
              <p className="pw-head__sub">{sub}</p>
            </div>
          </header>
          <div key={step} className="pw-body">
            <WizardStepBody wizard={wizard} uid={uid} />
          </div>
          <footer className="pw-foot">
            <span className="pw-foot__sp" />
            {isWizardItem(step) && (
              <button type="button" className="pw-skip" onClick={skip}>
                {t('settings.autoconfig.wizard.skip')}
              </button>
            )}
            <button
              type="button"
              className="btn btn-secondary pw-foot__btn"
              onClick={stepIndex === 0 ? handleClose : () => go(stepIndex - 1)}
            >
              {stepIndex === 0 ? t('common.cancel') : t('settings.autoconfig.wizard.back')}
            </button>
            {step === 'review' ? (
              <button type="button" className="btn btn-primary pw-foot__btn" onClick={create} disabled={saving}>
                {saving ? t('settings.autoconfig.saving') : t('settings.autoconfig.wizard.create')}
              </button>
            ) : (
              <button type="button" className="btn btn-primary pw-foot__btn" onClick={next} aria-disabled={blocked && attempted}>
                {t('settings.autoconfig.wizard.next')}
                <ArrowRight size={15} aria-hidden="true" />
              </button>
            )}
          </footer>
        </div>
      </div>
    </div>,
    document.body
  );
}
