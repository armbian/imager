// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useId, type RefObject } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { ArrowRight, Cable, Check, Globe, Info, KeyRound, Tag, User, Wifi, X } from 'lucide-react';
import { UI } from '../../config';
import { isWizardItem } from '../../config/profileWizard';
import { useDialogFocus } from '../../hooks/useDialogFocus';
import { useModalExitAnimation } from '../../hooks/useModalExitAnimation';
import { useProfileWizard, type ProfileWizardState } from '../../hooks/useProfileWizard';
import { FieldHint } from '../settings/editor/EditorParts';
import type { AutoconfigProfile, ProfileBoard } from '../../types';
import { WizardAccessStep } from './WizardAccessStep';
import { WizardLocaleStep } from './WizardLocaleStep';
import { WizardNameStep } from './WizardNameStep';
import { WizardNetworkStep } from './WizardNetworkStep';
import { WizardReviewStep } from './WizardReviewStep';
import { WizardSide } from './WizardSide';
import { WizardUserStep } from './WizardUserStep';

const STEP_ICON = { name: Tag, network: Wifi, user: User, locale: Globe, access: KeyRound, review: Check } as const;

interface ProfileWizardProps {
  /** The board picked in the flash flow, offered as the profile's scope; null from Settings */
  board: ProfileBoard | null;
  boardImage: string | null;
  /** Opened from the flash summary, where the created profile gets picked */
  fromFlash?: boolean;
  returnFocusRef?: RefObject<HTMLElement | null>;
  onClose: () => void;
  onCreated: (profile: AutoconfigProfile) => void;
}

function WizardStepBody({ wizard, uid, board }: { wizard: ProfileWizardState; uid: string; board: ProfileBoard | null }) {
  switch (wizard.step) {
    case 'name':
      return <WizardNameStep wizard={wizard} uid={uid} board={board} />;
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

export function ProfileWizard({ board, boardImage, fromFlash = false, returnFocusRef, onClose, onCreated }: ProfileWizardProps) {
  const { t } = useTranslation();
  const uid = useId();
  const titleId = `${uid}-title`;
  const { isExiting, handleClose } = useModalExitAnimation({ onClose });
  const wizard = useProfileWizard(board, (profile) => {
    onCreated(profile);
    handleClose();
  });
  const { model, step, stepIndex, saving, blocked, empty, attempted, go, next, skip, create } = wizard;
  const dialogRef = useDialogFocus<HTMLDivElement>({ onEscape: saving ? undefined : handleClose, returnFocusRef });

  const reviewEmpty = step === 'review' && empty;
  const HeadIcon = reviewEmpty
    ? Info
    : step === 'network' && model.network.apply && model.network.mode === 'cable'
      ? Cable
      : STEP_ICON[step];
  const title = step === 'review'
    ? t(reviewEmpty ? 'settings.autoconfig.wizard.review.titleEmpty' : 'settings.autoconfig.wizard.review.title', { name: model.name.trim() })
    : t(`settings.autoconfig.wizard.${step}.title`);
  const sub = reviewEmpty
    ? t('settings.autoconfig.wizard.review.subEmpty')
    : step === 'review' && !fromFlash
      ? t('settings.autoconfig.wizard.review.subSettings')
      : t(`settings.autoconfig.wizard.${step}.sub`);

  return createPortal(
    <div className={`modal-overlay pw-overlay ${isExiting ? 'modal-exiting' : 'modal-entering'}`} data-tauri-drag-region>
      <div
        ref={dialogRef}
        className={`pw-card${isExiting ? ' is-exiting' : ''}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <WizardSide wizard={wizard} boardImage={boardImage} />

        <div className="pw-main pe">
          <header className="pw-head">
            <span className="pw-head__tile" aria-hidden="true">
              <HeadIcon size={UI.ICON_SIZE.WIZARD_TILE} />
            </span>
            <div className="pw-head__text">
              <h2 id={titleId} className="pw-head__title">{title}</h2>
              <p className="pw-head__sub">{sub}</p>
            </div>
            <button type="button" className="pw-x" onClick={handleClose} aria-label={t('common.close')}>
              <X size={17} aria-hidden="true" />
            </button>
          </header>
          <div key={step} className="pw-body">
            <WizardStepBody wizard={wizard} uid={uid} board={board} />
          </div>
          <footer className="pw-foot">
            <span className="pw-foot__sp" aria-live="polite">
              {reviewEmpty && <FieldHint tone="warn">{t('settings.autoconfig.editor.createNeedsSomething')}</FieldHint>}
            </span>
            {isWizardItem(step) && (
              <button type="button" className="pw-skip" onClick={skip}>
                {t('settings.autoconfig.wizard.skip')}
              </button>
            )}
            <button
              type="button"
              className="btn btn-secondary btn-pill pw-foot__btn"
              onClick={stepIndex === 0 ? handleClose : () => go(stepIndex - 1)}
            >
              {stepIndex === 0 ? t('common.cancel') : t('settings.autoconfig.wizard.back')}
            </button>
            {step === 'review' ? (
              <button type="button" className="btn btn-primary btn-pill pw-foot__btn" onClick={create} disabled={saving} aria-disabled={empty || undefined}>
                {saving ? t('settings.autoconfig.saving') : t('settings.autoconfig.wizard.create')}
              </button>
            ) : (
              <button type="button" className="btn btn-primary btn-pill pw-foot__btn" onClick={next} aria-disabled={blocked && attempted}>
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
