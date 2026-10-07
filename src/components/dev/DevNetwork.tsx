// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useId } from 'react';
import { RotateCcw } from 'lucide-react';
import { DevNumberField, DevSegmented } from './DevControls';
import { DEV_API_FAULTS, DEV_DOWNLOAD_FAULTS, DEV_SCENARIOS, DEV_SLOW_FAULT } from '../../config/devScenarios';
import type { DevScenarioApi } from '../../hooks/useDevScenario';
import type { DevNetworkSim, DevScenariosStatus } from '../../types';

const { ICON_SIZE, DOWNLOAD_KB_PER_SEC } = DEV_SCENARIOS;

interface DevNetworkProps {
  dev: DevScenarioApi;
  status: DevScenariosStatus;
  onResetFlow: (() => void) | null;
}

export function DevNetwork({ dev, status, onResetFlow }: DevNetworkProps) {
  const hintId = useId();
  const { network } = status.scenario;

  const setNetwork = (change: Partial<DevNetworkSim>) =>
    dev.patch((s) => ({ ...s, network: { ...s.network, ...change } }));

  return (
    <div className="dev-stack">
      <DevSegmented
        label="API"
        options={DEV_API_FAULTS}
        value={network.api}
        disabled={dev.busy}
        onChange={(api) => setNetwork({ api })}
      />
      <DevSegmented
        label="Image download"
        options={DEV_DOWNLOAD_FAULTS}
        value={network.download}
        disabled={dev.busy}
        onChange={(download) => setNetwork({ download })}
      />
      <div className="dev-grid">
        <DevNumberField
          label="Slow API delay"
          unit="ms"
          value={network.apiDelayMs}
          min={0}
          max={status.limits.maxDelayMs}
          disabled={dev.busy || network.api !== DEV_SLOW_FAULT}
          onCommit={(apiDelayMs) => setNetwork({ apiDelayMs })}
        />
        <DevNumberField
          label="Slow download"
          unit="KB/s"
          value={network.downloadKbPerSec}
          min={DOWNLOAD_KB_PER_SEC.MIN}
          max={DOWNLOAD_KB_PER_SEC.MAX}
          disabled={dev.busy || network.download !== DEV_SLOW_FAULT}
          onCommit={(downloadKbPerSec) => setNetwork({ downloadKbPerSec })}
        />
      </div>
      <p className="dev-note" id={hintId}>
        {onResetFlow
          ? 'Lists load when their step opens. Restart the selection to go back to the first step and fetch it again under the new fault.'
          : 'A flash is running. Restart the selection once it finishes or fails.'}
      </p>
      <button
        type="button"
        className="dev-btn dev-btn--quiet dev-btn--block"
        disabled={!onResetFlow}
        aria-describedby={hintId}
        onClick={onResetFlow ?? undefined}
      >
        <RotateCcw size={ICON_SIZE.SM} aria-hidden="true" />
        Restart the selection
      </button>
    </div>
  );
}
