// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useId } from 'react';
import { Check } from 'lucide-react';
import { DEV_PRESET_FALLBACK, DEV_PRESET_ICONS, DEV_SCENARIOS, groupPresets } from '../../config/devScenarios';
import type { DevScenarioApi } from '../../hooks/useDevScenario';
import type { DevPreset, DevScenariosStatus } from '../../types';

const { ICON_SIZE } = DEV_SCENARIOS;

interface DevPresetsProps {
  dev: DevScenarioApi;
  status: DevScenariosStatus;
  active: DevPreset | null;
}

export function DevPresets({ dev, status, active }: DevPresetsProps) {
  const groupId = useId();
  return (
    <div className="dev-presets">
      {groupPresets(status.presets).map((group, g) => (
        <div key={group.label} className="dev-presets__group" role="group" aria-labelledby={`${groupId}-${g}`}>
          <span id={`${groupId}-${g}`} className="dev-field__label dev-presets__label">
            {group.label}
          </span>
          {group.presets.map((preset) => {
            const Icon = DEV_PRESET_ICONS[preset.id] ?? DEV_PRESET_FALLBACK.icon;
            const isActive = active?.id === preset.id;
            return (
              <button
                key={preset.id}
                type="button"
                className="dev-preset"
                aria-pressed={isActive}
                disabled={dev.busy}
                onClick={() => dev.applyPreset(preset)}
              >
                <span className="dev-preset__icon" aria-hidden="true">
                  <Icon size={ICON_SIZE.SM} />
                </span>
                <span className="dev-preset__text">
                  <span className="dev-preset__label">{preset.label}</span>
                  <span className="dev-preset__desc">{preset.description}</span>
                </span>
                <span className="dev-preset__check" aria-hidden="true">
                  {isActive && <Check size={ICON_SIZE.SM} />}
                </span>
              </button>
            );
          })}
        </div>
      ))}
    </div>
  );
}
