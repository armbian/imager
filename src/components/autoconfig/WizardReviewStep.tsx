// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useTranslation } from 'react-i18next';
import { WIZARD_ITEMS, WIZARD_STEPS, WIZARD_SUMMARY_SEPARATOR, wizardItemSummary } from '../../config/profileWizard';
import type { ProfileWizardState } from '../../hooks/useProfileWizard';

export function WizardReviewStep({ wizard }: { wizard: ProfileWizardState }) {
  const { t } = useTranslation();
  const { draft, go } = wizard;

  return (
    <>
      <dl className="pw-review">
        {WIZARD_ITEMS.map((item) => {
          const parts = wizardItemSummary(item, draft, t);
          return (
            <div key={item} className="pw-review__row">
              <dt>{t(`settings.autoconfig.wizard.steps.${item}`)}</dt>
              <dd>{parts.length > 0 ? parts.join(WIZARD_SUMMARY_SEPARATOR.REVIEW) : t(`settings.autoconfig.wizard.pending.${item}`)}</dd>
              <button type="button" className="pw-review__edit" onClick={() => go(WIZARD_STEPS.indexOf(item))}>
                {t('settings.autoconfig.edit')}
              </button>
            </div>
          );
        })}
      </dl>
      <p className="pw-line">{t('settings.autoconfig.wizard.review.later')}</p>
    </>
  );
}
