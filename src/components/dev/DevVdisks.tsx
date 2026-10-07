// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useId, useState } from 'react';
import { Crosshair, Database, Plus, Trash2 } from 'lucide-react';
import { DevEmpty, DevSegButtons } from './DevControls';
import { ConfirmationDialog } from '../shared';
import {
  asOnlyTarget,
  DEV_SCENARIOS,
  devSizeOptions,
  isValidDevId,
  isVdiskTarget,
  nextFreeId,
  takenDeviceIds,
} from '../../config/devScenarios';
import { formatBytes, tailPath } from '../../utils';
import type { DevScenarioApi } from '../../hooks/useDevScenario';
import type { DevScenariosStatus, DevVdisk } from '../../types';

const { ICON_SIZE, VDISK_SIZES_MB, DEFAULT_VDISK_MB, VDISK_ID_PREFIX, VDISK_PATH_SEGMENTS } = DEV_SCENARIOS;

interface DevVdisksProps {
  dev: DevScenarioApi;
  status: DevScenariosStatus;
}

export function DevVdisks({ dev, status }: DevVdisksProps) {
  const idInputId = useId();
  const [newId, setNewId] = useState('');
  const [sizeMb, setSizeMb] = useState<number>(DEFAULT_VDISK_MB);
  const [pendingDelete, setPendingDelete] = useState<DevVdisk | null>(null);
  const { scenario, vdisks, limits } = status;

  if (!status.vdiskSupported) {
    return (
      <DevEmpty
        icon={Database}
        title="Virtual disks need macOS"
        hint="They are sparse files written through the real flash path; use fake devices on this platform."
      />
    );
  }

  const taken = takenDeviceIds(scenario, vdisks);
  const suggestedId = nextFreeId(VDISK_ID_PREFIX, taken);
  const typedId = newId.trim();
  const nextId = typedId || suggestedId;
  const idValid = isValidDevId(nextId, limits.idMaxLen);
  const idTaken = taken.has(nextId);
  const idInvalid = typedId.length > 0 && (!idValid || idTaken);

  const create = async () => {
    if (await dev.createVdisk(nextId, sizeMb)) setNewId('');
  };

  return (
    <div className="dev-stack">
      {vdisks.length === 0 ? (
        <DevEmpty
          icon={Database}
          title="No virtual disks yet"
          hint="Create one below; flashing it writes a real file in the app cache."
        />
      ) : (
        <ul className="dev-devices">
          {vdisks.map((v) => (
            <li key={v.id} className="dev-device">
              <span className="dev-device__icon" aria-hidden="true">
                <Database size={ICON_SIZE.MD} />
              </span>
              <div className="dev-device__text">
                <span className="dev-device__title">{v.id}</span>
                <span className="dev-device__line">
                  <span className="dev-device__meta">{formatBytes(v.sizeBytes)}</span>
                  <span className="dev-device__meta dev-device__meta--more" title={v.path}>
                    {tailPath(v.path, VDISK_PATH_SEGMENTS)}
                  </span>
                  {isVdiskTarget(scenario, v.id) && <span className="dev-tag dev-tag--note dev-tag--target">flash target</span>}
                </span>
              </div>
              <div className="dev-device__actions">
                <button
                  type="button"
                  className="dev-icon-btn"
                  aria-label={`Use ${v.id} as the flash target`}
                  title="Flash target: hide real devices and list this disk"
                  aria-pressed={isVdiskTarget(scenario, v.id)}
                  disabled={dev.busy}
                  onClick={() => dev.setScenario(asOnlyTarget(scenario, v.id), `${v.id} is the flash target`)}
                >
                  <Crosshair size={ICON_SIZE.SM} />
                </button>
                <button
                  type="button"
                  className="dev-icon-btn dev-icon-btn--danger"
                  aria-label={`Delete ${v.id}`}
                  title="Delete the file"
                  disabled={dev.busy}
                  onClick={() => setPendingDelete(v)}
                >
                  <Trash2 size={ICON_SIZE.SM} />
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}

      <div className="dev-subblock">
        <label className="dev-field__label" htmlFor={idInputId}>
          New virtual disk
        </label>
        <div className="dev-row">
          <div className={`dev-input dev-input--grow${idInvalid ? ' is-invalid' : ''}`}>
            <input
              id={idInputId}
              type="text"
              value={newId}
              maxLength={limits.idMaxLen}
              placeholder={suggestedId}
              spellCheck={false}
              aria-invalid={idInvalid}
              onChange={(e) => setNewId(e.target.value.toLowerCase())}
            />
          </div>
          <button
            type="button"
            className="dev-btn"
            disabled={dev.busy || !idValid || idTaken}
            onClick={create}
          >
            <Plus size={ICON_SIZE.SM} aria-hidden="true" />
            Create
          </button>
        </div>
        <DevSegButtons
          label="Virtual disk size"
          options={devSizeOptions(VDISK_SIZES_MB, limits.maxVdiskMb)}
          value={sizeMb}
          onChange={setSizeMb}
        />
        <p className="dev-note">
          {idTaken
            ? 'That id is already in use.'
            : `Lowercase letters, digits and dashes, up to ${limits.idMaxLen}. Creating lists it; "Flash target" also hides real devices.`}
        </p>
      </div>

      <ConfirmationDialog
        isOpen={pendingDelete !== null}
        title="Delete virtual disk?"
        message={pendingDelete ? `${pendingDelete.path} (${formatBytes(pendingDelete.sizeBytes)}) is removed from disk and from the scenario.` : ''}
        cancelText="Cancel"
        confirmText="Delete"
        onCancel={() => setPendingDelete(null)}
        onConfirm={() => {
          const target = pendingDelete;
          setPendingDelete(null);
          if (target) dev.deleteVdisk(target.id);
        }}
      />
    </div>
  );
}
