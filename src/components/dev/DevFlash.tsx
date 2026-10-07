// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useState } from 'react';
import { FileDown } from 'lucide-react';
import { DevNumberField, DevRadioList, DevSegButtons, DevSwitch } from './DevControls';
import { DEV_FLASH_OUTCOMES, DEV_OUTCOMES_WITH_FAIL_POINT, DEV_SCENARIOS, devSizeOptions } from '../../config/devScenarios';
import { formatBytes, getErrorMessage, tailPath } from '../../utils';
import { useToasts } from '../../hooks/useToasts';
import type { DevScenarioApi } from '../../hooks/useDevScenario';
import type { CustomImageInfo, DevFlashSim, DevScenariosStatus } from '../../types';

const { ICON_SIZE, FAIL_AT_PERCENT, SPEED_MB_PER_SEC, TEST_IMAGE_SIZES_MB, DEFAULT_TEST_IMAGE_MB } = DEV_SCENARIOS;

interface DevFlashProps {
  dev: DevScenarioApi;
  status: DevScenariosStatus;
  isFlashing: boolean;
  onUseCustomImage: (image: CustomImageInfo) => Promise<void>;
}

export function DevFlash({ dev, status, isFlashing, onUseCustomImage }: DevFlashProps) {
  const { showSuccess, showError } = useToasts();
  const [sizeMb, setSizeMb] = useState<number>(DEFAULT_TEST_IMAGE_MB);
  const [unaligned, setUnaligned] = useState(false);
  const { flash } = status.scenario;
  const failPointApplies = DEV_OUTCOMES_WITH_FAIL_POINT.includes(flash.outcome);

  const setFlash = (change: Partial<DevFlashSim>) =>
    dev.patch((s) => ({ ...s, flash: { ...s.flash, ...change } }));

  const handleTestImage = async () => {
    const image = await dev.makeTestImage(sizeMb, unaligned);
    if (!image) return;
    const name = tailPath(image.path);
    try {
      await onUseCustomImage({ path: image.path, name, size: image.sizeBytes });
      showSuccess(`Test image selected: ${name} (${formatBytes(image.sizeBytes)})`);
    } catch (err) {
      showError(getErrorMessage(err, 'Could not select the test image'));
    }
  };

  return (
    <div className="dev-stack">
      <DevRadioList
        label="Outcome"
        options={DEV_FLASH_OUTCOMES}
        value={flash.outcome}
        disabled={dev.busy}
        onChange={(outcome) => setFlash({ outcome })}
      />
      <div className="dev-grid">
        <DevNumberField
          label="Fails at"
          unit="%"
          value={flash.failAtPercent}
          min={FAIL_AT_PERCENT.MIN}
          max={FAIL_AT_PERCENT.MAX}
          disabled={dev.busy || !failPointApplies}
          onCommit={(failAtPercent) => setFlash({ failAtPercent })}
        />
        <DevNumberField
          label="Auth delay"
          unit="ms"
          value={flash.authDelayMs}
          min={0}
          max={status.limits.maxDelayMs}
          disabled={dev.busy}
          onCommit={(authDelayMs) => setFlash({ authDelayMs })}
        />
        <DevNumberField
          label="Write"
          unit="MB/s"
          value={flash.writeMbPerSec}
          min={SPEED_MB_PER_SEC.MIN}
          max={SPEED_MB_PER_SEC.MAX}
          disabled={dev.busy}
          onCommit={(writeMbPerSec) => setFlash({ writeMbPerSec })}
        />
        <DevNumberField
          label="Verify"
          unit="MB/s"
          value={flash.verifyMbPerSec}
          min={SPEED_MB_PER_SEC.MIN}
          max={SPEED_MB_PER_SEC.MAX}
          disabled={dev.busy}
          onCommit={(verifyMbPerSec) => setFlash({ verifyMbPerSec })}
        />
      </div>
      <p className="dev-note">Outcomes apply to fake devices only; a virtual disk always writes for real.</p>

      <div className="dev-subblock">
        <span className="dev-field__label">Test image</span>
        <DevSegButtons
          label="Test image size"
          options={devSizeOptions(TEST_IMAGE_SIZES_MB, status.limits.maxTestImageMb)}
          value={sizeMb}
          onChange={setSizeMb}
        />
        <DevSwitch
          label="Unaligned size"
          hint="Ends mid-sector, to test padding."
          checked={unaligned}
          onChange={setUnaligned}
        />
        <button type="button" className="dev-btn dev-btn--block" disabled={dev.busy || isFlashing} onClick={handleTestImage}>
          <FileDown size={ICON_SIZE.SM} aria-hidden="true" />
          Make and use as custom image
        </button>
      </div>
    </div>
  );
}
