// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

// Never re-export from config/index.ts: these names would leak into release bundles (CI greps dist/).

import {
  Ban,
  Database,
  Gauge,
  Globe,
  Hash,
  HardDrive,
  Inbox,
  LayoutList,
  Lock,
  LogOut,
  MemoryStick,
  Palette,
  Plug,
  Server,
  ShieldX,
  TriangleAlert,
  Unplug,
  Usb,
  WifiOff,
  Zap,
  type LucideIcon,
} from 'lucide-react';
import { AUTO_LANGUAGE_CODE, SUPPORTED_LANGUAGES } from './i18n';
import { BYTES_PER_MB, PLATFORM, SETTINGS } from './constants';
import { formatBytes, uiPlatform } from '../utils';
import type { Theme } from '../contexts/ThemeContext';
import type { MotionMode } from '../contexts/MotionContext';
import type {
  DevApiFault,
  DevDownloadFault,
  DevFakeDevice,
  DevFakeEdlDevice,
  DevFlashOutcome,
  DevPreset,
  DevScenario,
  DevScenariosStatus,
  DevVdisk,
} from '../types';

export const DEV_SCENARIOS = {
  ROOT_ID: 'armbian-dev-scenarios',
  DRAWER_ID: 'armbian-dev-scenarios-drawer',
  TITLE: 'Dev scenarios',
  STATUS_POLL_MS: 2000,
  SHORTCUT_KEY: 'd',
  SHORTCUT_LABEL: { MAC: '⌘⇧D', OTHER: 'Ctrl+Shift+D' },
  WINDOWS_ROOT_CLASS: 'dev-root--windows',
  /** Mirrors config::flash::SIMULATED_DEVICE_PREFIX in src-tauri/src/config/mod.rs */
  DEVICE_PREFIX: 'devsim://',
  MS_PER_SECOND: 1000,
  ICON_SIZE: { SM: 13, MD: 15, LG: 18 },
  /** Mirrors dev::VDISK_MODEL in src-tauri/src/config/mod.rs */
  VDISK_MODEL: 'Virtual disk (file)',
  HOT_PLUG_MS: 5000,
  FAIL_AT_PERCENT: { MIN: 1, MAX: 99 },
  SPEED_MB_PER_SEC: { MIN: 1, MAX: 10_000 },
  DOWNLOAD_KB_PER_SEC: { MIN: 1, MAX: 1_000_000 },
  TEST_IMAGE_SIZES_MB: [64, 512, 2048],
  DEFAULT_TEST_IMAGE_MB: 64,
  VDISK_SIZES_MB: [512, 2048, 8192],
  DEFAULT_VDISK_MB: 2048,
  VDISK_ID_PREFIX: 'vdisk',
  VDISK_PATH_SEGMENTS: 2,
  ID_PATTERN: /^[a-z0-9-]+$/,
} as const;

export const DEV_STORAGE_KEYS = {
  SECTION: 'armbian-dev-scenarios:section',
} as const;

export const DEV_SECTION = {
  PRESETS: 'presets',
  DEVICES: 'devices',
  FLASH: 'flash',
  VDISKS: 'vdisks',
  NETWORK: 'network',
  APP: 'app',
} as const;

export type DevSectionId = (typeof DEV_SECTION)[keyof typeof DEV_SECTION];

export interface DevSectionInfo {
  id: DevSectionId;
  label: string;
  title: string;
  icon: LucideIcon;
}

export const DEV_SECTIONS: DevSectionInfo[] = [
  { id: DEV_SECTION.PRESETS, label: 'Presets', title: 'Presets', icon: LayoutList },
  { id: DEV_SECTION.DEVICES, label: 'Devices', title: 'Devices', icon: HardDrive },
  { id: DEV_SECTION.FLASH, label: 'Flash', title: 'Flash outcome', icon: Zap },
  { id: DEV_SECTION.VDISKS, label: 'Disks', title: 'Virtual disks', icon: Database },
  { id: DEV_SECTION.NETWORK, label: 'Network', title: 'Network & API', icon: Globe },
  { id: DEV_SECTION.APP, label: 'App', title: 'App', icon: Palette },
];

export function isDevSectionId(value: unknown): value is DevSectionId {
  return DEV_SECTIONS.some((s) => s.id === value);
}

export const DEV_RAIL_KEY_STEP: Record<string, number> = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 };

export function railIndexForKey(key: string, index: number, count: number): number | null {
  if (key === 'Home') return 0;
  if (key === 'End') return count - 1;
  if (key in DEV_RAIL_KEY_STEP) return (index + DEV_RAIL_KEY_STEP[key] + count) % count;
  return null;
}

export function devShortcutLabel(): string {
  return uiPlatform() === PLATFORM.MACOS ? DEV_SCENARIOS.SHORTCUT_LABEL.MAC : DEV_SCENARIOS.SHORTCUT_LABEL.OTHER;
}

/** The Windows 11 frame radius needs its own root class; main.tsx tags only macOS. */
export function devRootPlatformClass(): string {
  return uiPlatform() === PLATFORM.WINDOWS ? DEV_SCENARIOS.WINDOWS_ROOT_CLASS : '';
}

/** Mirrors the first preset in src-tauri/src/dev_scenarios/presets.rs */
export const DEV_DEFAULT_PRESET_ID = 'clean';

export const DEV_PRESET_GROUPS: { label: string; ids: string[] }[] = [
  { label: 'Off', ids: [DEV_DEFAULT_PRESET_ID] },
  { label: 'Devices', ids: ['sd-card', 'mixed-devices', 'read-only-sd', 'too-small', 'hot-plug'] },
  { label: 'Flash', ids: ['write-error', 'verify-mismatch', 'unplug-mid-write', 'auth-denied', 'slow-flash'] },
  { label: 'Qualcomm EDL', ids: ['edl-device', 'edl-unplug'] },
  { label: 'Network', ids: ['offline', 'slow-network', 'empty-lists', 'server-error', 'sha-mismatch'] },
];

export const DEV_PRESET_ICONS: Record<string, LucideIcon> = {
  [DEV_DEFAULT_PRESET_ID]: Ban,
  'sd-card': MemoryStick,
  'mixed-devices': HardDrive,
  'read-only-sd': Lock,
  'too-small': TriangleAlert,
  'hot-plug': Plug,
  'write-error': Zap,
  'verify-mismatch': TriangleAlert,
  'unplug-mid-write': LogOut,
  'auth-denied': ShieldX,
  'slow-flash': Gauge,
  'edl-device': Usb,
  'edl-unplug': Unplug,
  offline: WifiOff,
  'slow-network': Gauge,
  'empty-lists': Inbox,
  'server-error': Server,
  'sha-mismatch': Hash,
};

export const DEV_PRESET_FALLBACK = { group: 'Other', icon: LayoutList } as const;

export function groupPresets(presets: DevPreset[]): { label: string; presets: DevPreset[] }[] {
  const known = new Set(DEV_PRESET_GROUPS.flatMap((g) => g.ids));
  const groups = DEV_PRESET_GROUPS.map((g) => ({
    label: g.label,
    presets: presets.filter((p) => g.ids.includes(p.id)),
  }));
  groups.push({ label: DEV_PRESET_FALLBACK.group, presets: presets.filter((p) => !known.has(p.id)) });
  return groups.filter((g) => g.presets.length > 0);
}

export interface DevOption<T extends string | number> {
  value: T;
  label: string;
}

export const DEV_FLASH_OUTCOMES: DevOption<DevFlashOutcome>[] = [
  { value: 'success', label: 'Success' },
  { value: 'writeError', label: 'Write error' },
  { value: 'verifyMismatch', label: 'Verify mismatch' },
  { value: 'unplug', label: 'Unplug' },
  { value: 'authDenied', label: 'Auth denied' },
];

export const DEV_OUTCOMES_WITH_FAIL_POINT: readonly DevFlashOutcome[] = ['writeError', 'verifyMismatch', 'unplug'];

export const DEV_SLOW_FAULT = 'slow' satisfies DevApiFault & DevDownloadFault;

export const DEV_API_FAULTS: DevOption<DevApiFault>[] = [
  { value: 'normal', label: 'Normal' },
  { value: 'offline', label: 'Offline' },
  { value: DEV_SLOW_FAULT, label: 'Slow' },
  { value: 'empty', label: 'Empty' },
  { value: 'serverError', label: 'HTTP 500' },
];

export const DEV_DOWNLOAD_FAULTS: DevOption<DevDownloadFault>[] = [
  { value: 'normal', label: 'Normal' },
  { value: 'offline', label: 'Offline' },
  { value: DEV_SLOW_FAULT, label: 'Slow' },
  { value: 'shaMismatch', label: 'Bad SHA' },
];

export const DEV_THEME_OPTIONS: DevOption<Theme>[] = [
  { value: SETTINGS.THEME_MODES.LIGHT, label: 'Light' },
  { value: SETTINGS.THEME_MODES.DARK, label: 'Dark' },
  { value: SETTINGS.THEME_MODES.AUTO, label: 'Auto' },
];

export const DEV_MOTION_OPTIONS: DevOption<MotionMode>[] = [
  { value: SETTINGS.MOTION_MODES.FULL, label: 'Full' },
  { value: SETTINGS.MOTION_MODES.REDUCE, label: 'Reduce' },
  { value: SETTINGS.MOTION_MODES.AUTO, label: 'Auto' },
];

export const DEV_PREVIEW_LANGUAGES = SUPPORTED_LANGUAGES.filter((l) => l.code !== AUTO_LANGUAGE_CODE);

export function devSizeOptions(sizesMb: readonly number[], maxMb: number): DevOption<number>[] {
  return sizesMb
    .filter((mb) => mb <= maxMb)
    .map((mb) => ({ value: mb, label: formatBytes(mb * BYTES_PER_MB) }));
}

export function devStatusText(active: boolean, presetLabel: string | null, updateSimulated: boolean): string {
  if (active) return `Simulating: ${presetLabel ?? 'custom scenario'}`;
  if (updateSimulated) return 'Simulating: an update offer';
  return 'Idle: real devices, flashing and network';
}

export interface DevDeviceTemplate {
  key: string;
  label: string;
  idPrefix: string;
  device: Omit<DevFakeDevice, 'id'>;
}

export const DEV_DEVICE_TEMPLATES: DevDeviceTemplate[] = [
  {
    key: 'sd',
    label: 'SD 32 GB',
    idPrefix: 'sd',
    device: { model: 'Simulated SD card', sizeBytes: 31_914_983_424, busType: 'SD', isRemovable: true },
  },
  {
    key: 'usb',
    label: 'USB 16 GB',
    idPrefix: 'usb',
    device: { model: 'Simulated USB stick', sizeBytes: 15_745_024_000, busType: 'USB', isRemovable: true },
  },
  {
    key: 'locked',
    label: 'Locked SD',
    idPrefix: 'sd-locked',
    device: { model: 'Simulated SD card (locked)', sizeBytes: 7_948_206_080, busType: 'SD', isRemovable: true, isReadOnly: true },
  },
  {
    key: 'small',
    label: 'SD 1 GiB',
    idPrefix: 'sd-small',
    device: { model: 'Simulated SD card (1 GiB)', sizeBytes: 1_073_741_824, busType: 'SD', isRemovable: true },
  },
  {
    key: 'nvme',
    label: 'System NVMe',
    idPrefix: 'nvme',
    device: { model: 'Simulated internal NVMe', sizeBytes: 512_110_190_592, busType: 'NVMe', isSystem: true },
  },
];

/** Mirrors dev::EDL_ID_PREFIX in src-tauri/src/config/mod.rs */
export const DEV_EDL_ID_PREFIX = 'edl-';

export const DEV_EDL_TEMPLATE: Omit<DevFakeEdlDevice, 'id'> & { idPrefix: string; serialPrefix: string } = {
  idPrefix: `${DEV_EDL_ID_PREFIX}board`,
  serialPrefix: 'SIM',
  description: 'Simulated Qualcomm EDL device',
};

export const DEV_DEVICE_TAG = {
  DISK: 'disk',
  SYSTEM: 'system',
  VDISK: 'vdisk',
  EDL: 'EDL',
} as const;

export const DEV_SIMULATED_UPDATE = {
  RID: -1,
  VERSION: '99.0.0',
  BODY: 'Simulated release from the dev scenarios panel. Installing it never relaunches the app.',
  CHUNKS: 20,
  CHUNK_BYTES: 2 * BYTES_PER_MB,
  CHUNK_INTERVAL_MS: 150,
  FAIL_AFTER_CHUNKS: 8,
  ERROR: 'Simulated update download failed',
} as const;

function sameJson(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

export function changedSections(status: DevScenariosStatus, updateSimulated: boolean): Set<DevSectionId> {
  const { scenario, presets } = status;
  const base = presets.find((p) => p.id === DEV_DEFAULT_PRESET_ID)?.scenario;
  const changed = new Set<DevSectionId>();
  if (status.active) changed.add(DEV_SECTION.PRESETS);
  if (base) {
    const devices = (s: DevScenario) => [s.hideRealDevices, s.devices, s.edlDevices];
    if (!sameJson(devices(scenario), devices(base))) changed.add(DEV_SECTION.DEVICES);
    if (!sameJson(scenario.flash, base.flash)) changed.add(DEV_SECTION.FLASH);
    if (!sameJson(scenario.network, base.network)) changed.add(DEV_SECTION.NETWORK);
  }
  if (scenario.devices.some((d) => d.vdisk)) changed.add(DEV_SECTION.VDISKS);
  if (updateSimulated) changed.add(DEV_SECTION.APP);
  return changed;
}

export function matchingPreset(scenario: DevScenario, presets: DevPreset[]): DevPreset | null {
  return presets.find((p) => sameJson(p.scenario, scenario)) ?? null;
}

export function takenDeviceIds(scenario: DevScenario, vdisks: DevVdisk[] = []): Set<string> {
  return new Set([...scenario.devices, ...scenario.edlDevices, ...vdisks].map((d) => d.id));
}

export function nextFreeId(prefix: string, taken: Set<string>): string {
  let n = 1;
  while (taken.has(`${prefix}-${n}`)) n++;
  return `${prefix}-${n}`;
}

export function isValidDevId(id: string, maxLen: number): boolean {
  return id.length <= maxLen && DEV_SCENARIOS.ID_PATTERN.test(id);
}

export function fakeDeviceCount(scenario: DevScenario): number {
  return scenario.devices.length + scenario.edlDevices.length;
}

export function newFakeDevice(template: DevDeviceTemplate, scenario: DevScenario, appearAfterMs: number | null): DevFakeDevice {
  return { ...template.device, id: nextFreeId(template.idPrefix, takenDeviceIds(scenario)), appearAfterMs };
}

export function newEdlDevice(scenario: DevScenario, appearAfterMs: number | null): DevFakeEdlDevice {
  const id = nextFreeId(DEV_EDL_TEMPLATE.idPrefix, takenDeviceIds(scenario));
  return {
    id,
    serial: `${DEV_EDL_TEMPLATE.serialPrefix}-${id}`,
    description: DEV_EDL_TEMPLATE.description,
    appearAfterMs,
  };
}

export function editById<T extends { id: string }>(items: T[], id: string, edit: (item: T) => T | null): T[] {
  return items.flatMap((item) => (item.id === id ? (edit(item) ?? []) : [item]));
}

export function leaveSoon<T extends DevFakeDevice | DevFakeEdlDevice>(device: T): T {
  return { ...device, appearAfterMs: null, disappearAfterMs: DEV_SCENARIOS.HOT_PLUG_MS };
}

export function fakeDeviceTags(device: DevFakeDevice): string[] {
  return [
    device.busType ?? DEV_DEVICE_TAG.DISK,
    ...(device.isSystem ? [DEV_DEVICE_TAG.SYSTEM] : []),
    ...(device.vdisk ? [DEV_DEVICE_TAG.VDISK] : []),
  ];
}

/** Mirrors vdisk_device() in src-tauri/src/commands/dev_scenarios.rs */
export function vdiskDevice(id: string): DevFakeDevice {
  return { id, model: DEV_SCENARIOS.VDISK_MODEL, sizeBytes: 0, busType: 'USB', isRemovable: true, vdisk: true };
}

function listsVdisk(scenario: DevScenario, id: string): boolean {
  return scenario.devices.some((d) => d.vdisk && d.id === id);
}

export function isVdiskTarget(scenario: DevScenario, id: string): boolean {
  return scenario.hideRealDevices && listsVdisk(scenario, id);
}

/** Real devices hidden and the vdisk listed: the only shape in which a flash may land on it. */
export function asOnlyTarget(scenario: DevScenario, id: string): DevScenario {
  return {
    ...scenario,
    hideRealDevices: true,
    devices: listsVdisk(scenario, id) ? scenario.devices : [...scenario.devices, vdiskDevice(id)],
  };
}

export type DevHotPlugState =
  | { kind: 'listed' }
  | { kind: 'arriving'; inMs: number }
  | { kind: 'leaving'; inMs: number }
  | { kind: 'gone' }
  | { kind: 'unplugged' };

/** Mirrors visible_at in src-tauri/src/dev_scenarios/model.rs */
export function hotPlugState(
  device: Pick<DevFakeDevice, 'id' | 'appearAfterMs' | 'disappearAfterMs'>,
  elapsedMs: number,
  unplugged: string[]
): DevHotPlugState {
  if (unplugged.includes(device.id)) return { kind: 'unplugged' };
  const appear = device.appearAfterMs ?? null;
  const disappear = device.disappearAfterMs ?? null;
  if (appear !== null && elapsedMs < appear) return { kind: 'arriving', inMs: appear - elapsedMs };
  if (disappear !== null && elapsedMs >= disappear) return { kind: 'gone' };
  if (disappear !== null) return { kind: 'leaving', inMs: disappear - elapsedMs };
  return { kind: 'listed' };
}

export function isHotPlugPresent(state: DevHotPlugState): boolean {
  return state.kind === 'listed' || state.kind === 'leaving';
}

export function formatSeconds(ms: number): string {
  return `${Math.max(1, Math.ceil(ms / DEV_SCENARIOS.MS_PER_SECOND))} s`;
}

export function hotPlugText(state: DevHotPlugState): string | null {
  switch (state.kind) {
    case 'arriving':
      return `arrives in ${formatSeconds(state.inMs)}`;
    case 'leaving':
      return `leaves in ${formatSeconds(state.inMs)}`;
    case 'gone':
      return 'left';
    case 'unplugged':
      return 'unplugged by flash';
    default:
      return null;
  }
}
