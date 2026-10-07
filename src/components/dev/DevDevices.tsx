// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useState } from 'react';
import { HardDrive, Lock, LockOpen, LogOut, Plus, Usb, X } from 'lucide-react';
import { DevEmpty, DevSwitch } from './DevControls';
import {
  DEV_DEVICE_TAG,
  DEV_DEVICE_TEMPLATES,
  DEV_SCENARIOS,
  editById,
  fakeDeviceCount,
  fakeDeviceTags,
  formatSeconds,
  hotPlugState,
  hotPlugText,
  isHotPlugPresent,
  leaveSoon,
  newEdlDevice,
  newFakeDevice,
  type DevDeviceTemplate,
  type DevHotPlugState,
} from '../../config/devScenarios';
import { formatBytes } from '../../utils';
import type { DevScenarioApi } from '../../hooks/useDevScenario';
import type { DevFakeDevice, DevFakeEdlDevice, DevScenariosStatus } from '../../types';

const { ICON_SIZE, HOT_PLUG_MS, DEVICE_PREFIX } = DEV_SCENARIOS;

interface RowProps {
  id: string;
  title: string;
  meta: string;
  edl: boolean;
  tags: string[];
  state: DevHotPlugState;
  busy: boolean;
  onToggleLock?: () => void;
  locked?: boolean;
  onLeave: () => void;
  onRemove: () => void;
}

function DeviceRow({ id, title, meta, edl, tags, state, busy, onToggleLock, locked, onLeave, onRemove }: RowProps) {
  const plug = hotPlugText(state);
  const Icon = edl ? Usb : HardDrive;
  return (
    <li className={`dev-device${isHotPlugPresent(state) ? '' : ' is-absent'}`}>
      <span className="dev-device__icon" aria-hidden="true">
        <Icon size={ICON_SIZE.MD} />
      </span>
      <div className="dev-device__text">
        <span className="dev-device__title">{title}</span>
        <span className="dev-device__line">
          <span className="dev-device__meta">
            {DEVICE_PREFIX}
            {id}
          </span>
          <span className="dev-device__meta dev-device__meta--more">{meta}</span>
          {tags.map((tag) => (
            <span key={tag} className={`dev-tag${tag === DEV_DEVICE_TAG.SYSTEM ? ' dev-tag--system' : ''}`}>
              {tag}
            </span>
          ))}
          {plug && <span className="dev-tag dev-tag--note dev-tag--plug">{plug}</span>}
        </span>
      </div>
      <div className="dev-device__actions">
        {onToggleLock && (
          <button
            type="button"
            className="dev-icon-btn"
            aria-label={locked ? `Unlock ${id}` : `Write-protect ${id}`}
            aria-pressed={locked}
            title={locked ? 'Unlock' : 'Write-protect'}
            disabled={busy}
            onClick={onToggleLock}
          >
            {locked ? <Lock size={ICON_SIZE.SM} /> : <LockOpen size={ICON_SIZE.SM} />}
          </button>
        )}
        <button
          type="button"
          className="dev-icon-btn"
          aria-label={`Unplug ${id} in ${formatSeconds(HOT_PLUG_MS)}`}
          title={`Unplug in ${formatSeconds(HOT_PLUG_MS)}`}
          disabled={busy}
          onClick={onLeave}
        >
          <LogOut size={ICON_SIZE.SM} />
        </button>
        <button
          type="button"
          className="dev-icon-btn"
          aria-label={`Remove ${id}`}
          title="Remove"
          disabled={busy}
          onClick={onRemove}
        >
          <X size={ICON_SIZE.SM} />
        </button>
      </div>
    </li>
  );
}

interface DevDevicesProps {
  dev: DevScenarioApi;
  status: DevScenariosStatus;
}

export function DevDevices({ dev, status }: DevDevicesProps) {
  const [arriveLater, setArriveLater] = useState(false);
  const { scenario, elapsedMs, unplugged, limits } = status;
  const full = fakeDeviceCount(scenario) >= limits.maxFakeDevices;
  const appearAfterMs = arriveLater ? HOT_PLUG_MS : null;

  const editDevice = (id: string, edit: (d: DevFakeDevice) => DevFakeDevice | null) =>
    dev.patch((s) => ({ ...s, devices: editById(s.devices, id, edit) }));

  const editEdl = (id: string, edit: (d: DevFakeEdlDevice) => DevFakeEdlDevice | null) =>
    dev.patch((s) => ({ ...s, edlDevices: editById(s.edlDevices, id, edit) }));

  const addDevice = (template: DevDeviceTemplate) =>
    dev.patch((s) => ({ ...s, devices: [...s.devices, newFakeDevice(template, s, appearAfterMs)] }));

  const addEdl = () => dev.patch((s) => ({ ...s, edlDevices: [...s.edlDevices, newEdlDevice(s, appearAfterMs)] }));

  return (
    <div className="dev-stack">
      <DevSwitch
        label="Hide real devices"
        hint="Lists only the fake ones and refuses any real flash target."
        checked={scenario.hideRealDevices}
        disabled={dev.busy}
        onChange={(hideRealDevices) => dev.patch((s) => ({ ...s, hideRealDevices }))}
      />

      {fakeDeviceCount(scenario) === 0 ? (
        <DevEmpty icon={HardDrive} title="No fake devices" hint="Add one below, or pick a preset." />
      ) : (
        <ul className="dev-devices">
          {scenario.devices.map((d) => (
            <DeviceRow
              key={d.id}
              id={d.id}
              title={d.model || d.id}
              meta={d.vdisk ? 'file-backed' : formatBytes(d.sizeBytes ?? 0)}
              edl={false}
              tags={fakeDeviceTags(d)}
              state={hotPlugState(d, elapsedMs, unplugged)}
              busy={dev.busy}
              locked={!!d.isReadOnly}
              onToggleLock={d.vdisk ? undefined : () => editDevice(d.id, (x) => ({ ...x, isReadOnly: !x.isReadOnly }))}
              onLeave={() => editDevice(d.id, leaveSoon)}
              onRemove={() => editDevice(d.id, () => null)}
            />
          ))}
          {scenario.edlDevices.map((d) => (
            <DeviceRow
              key={d.id}
              id={d.id}
              title={d.description || d.id}
              meta={d.serial ? `serial ${d.serial}` : 'EDL'}
              edl
              tags={[DEV_DEVICE_TAG.EDL]}
              state={hotPlugState(d, elapsedMs, unplugged)}
              busy={dev.busy}
              onLeave={() => editEdl(d.id, leaveSoon)}
              onRemove={() => editEdl(d.id, () => null)}
            />
          ))}
        </ul>
      )}

      <div className="dev-add">
        <span className="dev-field__label">Add a device</span>
        <div className="dev-chips">
          {DEV_DEVICE_TEMPLATES.map((template) => (
            <button
              key={template.key}
              type="button"
              className="dev-chip"
              disabled={dev.busy || full}
              onClick={() => addDevice(template)}
            >
              <Plus size={ICON_SIZE.SM} aria-hidden="true" />
              {template.label}
            </button>
          ))}
          <button type="button" className="dev-chip" disabled={dev.busy || full} onClick={addEdl}>
            <Plus size={ICON_SIZE.SM} aria-hidden="true" />
            EDL board
          </button>
        </div>
        <label className="dev-check">
          <input type="checkbox" checked={arriveLater} onChange={(e) => setArriveLater(e.target.checked)} />
          Arrives {formatSeconds(HOT_PLUG_MS)} after it is added
        </label>
        <p className="dev-note">
          {full
            ? `At most ${limits.maxFakeDevices} fake devices.`
            : 'Every change restarts the hot-plug clock and brings unplugged devices back.'}
        </p>
      </div>
    </div>
  );
}
