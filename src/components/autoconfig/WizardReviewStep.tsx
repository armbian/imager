// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Cpu, Layers } from 'lucide-react';
import { UI } from '../../config';
import { profileScopeLabel, type ProfileFact } from '../../config/profileEditorModel';
import { WIZARD_ITEMS, WIZARD_STEPS, wizardItemFact } from '../../config/profileWizard';
import type { ProfileWizardState } from '../../hooks/useProfileWizard';
import { WizardItemIcon } from './WizardSide';

interface ReviewRowProps {
  icon: ReactNode;
  label: string;
  value: string;
  tone?: ProfileFact['tone'];
  onEdit: () => void;
}

function ReviewRow({ icon, label, value, tone = 'ok', onEdit }: ReviewRowProps) {
  const { t } = useTranslation();
  return (
    <div className={`pw-review__row is-${tone}`}>
      <dt>
        <span className="pw-review__icon" aria-hidden="true">
          {icon}
        </span>
        {label}
      </dt>
      <dd className={`is-${tone}`}>
        <span className="pw-review__value">{value}</span>
      </dd>
      <button type="button" className="pw-review__edit" onClick={onEdit}>
        {t('settings.autoconfig.edit')}
      </button>
    </div>
  );
}

export function WizardReviewStep({ wizard }: { wizard: ProfileWizardState }) {
  const { t } = useTranslation();
  const { model, facts, go } = wizard;
  const scoped = model.scope === 'some' ? model.boards : [];
  const ScopeIcon = scoped.length ? Cpu : Layers;

  return (
    <dl className="pw-review">
      <ReviewRow
        icon={<ScopeIcon size={UI.ICON_SIZE.WIZARD_ITEM} />}
        label={t('settings.autoconfig.editor.offeredFor')}
        value={profileScopeLabel(scoped, t)}
        onEdit={() => go(0)}
      />
      {WIZARD_ITEMS.map((item) => {
        const fact = wizardItemFact(item, facts);
        return (
          <ReviewRow
            key={item}
            icon={<WizardItemIcon item={item} cable={model.network.apply && model.network.mode === 'cable'} />}
            label={t(`settings.autoconfig.wizard.steps.${item}`)}
            value={fact.text}
            tone={fact.tone}
            onEdit={() => go(WIZARD_STEPS.indexOf(item))}
          />
        );
      })}
    </dl>
  );
}
