// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useRef, useState, type KeyboardEvent, type Ref } from 'react';
import { Keyboard, Power, X } from 'lucide-react';
import { DevPresets } from './DevPresets';
import { DevDevices } from './DevDevices';
import { DevFlash } from './DevFlash';
import { DevVdisks } from './DevVdisks';
import { DevNetwork } from './DevNetwork';
import { DevApp } from './DevApp';
import {
  changedSections,
  DEV_SCENARIOS,
  DEV_SECTION,
  DEV_SECTIONS,
  DEV_STORAGE_KEYS,
  devStatusText,
  isDevSectionId,
  matchingPreset,
  railIndexForKey,
  type DevSectionId,
} from '../../config/devScenarios';
import { readLocalJson, writeLocalJson } from '../../utils/storage';
import type { DevScenarioApi } from '../../hooks/useDevScenario';
import type { CustomImageInfo, DevScenariosStatus } from '../../types';

const { ICON_SIZE, DRAWER_ID, TITLE } = DEV_SCENARIOS;
const PANEL_ID = `${DRAWER_ID}-panel`;
const tabId = (id: DevSectionId) => `${DRAWER_ID}-tab-${id}`;

function initialSection(): DevSectionId {
  const stored = readLocalJson<unknown>(DEV_STORAGE_KEYS.SECTION, null);
  return isDevSectionId(stored) ? stored : DEV_SECTION.PRESETS;
}

interface DevDrawerProps {
  dev: DevScenarioApi;
  status: DevScenariosStatus;
  compact: boolean;
  isExiting: boolean;
  isFlashing: boolean;
  updateSimulated: boolean;
  shortcut: string;
  onClose: () => void;
  onUseCustomImage: (image: CustomImageInfo) => Promise<void>;
  onResetFlow: (() => void) | null;
  ref?: Ref<HTMLDivElement>;
}

// Non-modal: the app stays usable behind it, so it never traps focus
export function DevDrawer({
  dev,
  status,
  compact,
  isExiting,
  isFlashing,
  updateSimulated,
  shortcut,
  onClose,
  onUseCustomImage,
  onResetFlow,
  ref,
}: DevDrawerProps) {
  const [section, setSection] = useState<DevSectionId>(initialSection);
  const tabsRef = useRef<(HTMLButtonElement | null)[]>([]);

  const select = (id: DevSectionId) => {
    setSection(id);
    writeLocalJson(DEV_STORAGE_KEYS.SECTION, id);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') {
      e.stopPropagation();
      onClose();
    }
  };

  const onRailKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const index = DEV_SECTIONS.findIndex((s) => s.id === section);
    const next = railIndexForKey(e.key, index, DEV_SECTIONS.length);
    if (next === null) return;
    e.preventDefault();
    select(DEV_SECTIONS[next].id);
    tabsRef.current[next]?.focus();
  };

  const { scenario, active } = status;
  const preset = matchingPreset(scenario, status.presets);
  const changed = changedSections(status, updateSimulated);
  const current = DEV_SECTIONS.find((s) => s.id === section) ?? DEV_SECTIONS[0];

  const content = () => {
    switch (current.id) {
      case DEV_SECTION.DEVICES:
        return <DevDevices dev={dev} status={status} />;
      case DEV_SECTION.FLASH:
        return <DevFlash dev={dev} status={status} isFlashing={isFlashing} onUseCustomImage={onUseCustomImage} />;
      case DEV_SECTION.VDISKS:
        return <DevVdisks dev={dev} status={status} />;
      case DEV_SECTION.NETWORK:
        return <DevNetwork dev={dev} status={status} onResetFlow={onResetFlow} />;
      case DEV_SECTION.APP:
        return <DevApp />;
      default:
        return <DevPresets dev={dev} status={status} active={preset} />;
    }
  };

  return (
    <div
      ref={ref}
      id={DRAWER_ID}
      role="dialog"
      aria-modal="false"
      aria-label={TITLE}
      tabIndex={-1}
      className={`dev-drawer${compact ? ' dev-drawer--compact' : ''}${isExiting ? ' is-exiting' : ''}`}
      onKeyDown={onKeyDown}
    >
      <nav className="dev-rail" aria-label="Dev scenario sections">
        <span className="dev-rail__badge" aria-hidden="true">
          DEV
        </span>
        <div className="dev-rail__tabs" role="tablist" aria-orientation="vertical" onKeyDown={onRailKeyDown}>
          {DEV_SECTIONS.map(({ id, label, icon: Icon }, i) => {
            const selected = id === section;
            const dot = changed.has(id) && !selected;
            return (
              <button
                key={id}
                ref={(el) => {
                  tabsRef.current[i] = el;
                }}
                id={tabId(id)}
                type="button"
                role="tab"
                className="dev-rail__tab"
                aria-selected={selected}
                aria-controls={PANEL_ID}
                aria-label={dot ? `${label}, changed` : label}
                tabIndex={selected ? 0 : -1}
                onClick={() => select(id)}
              >
                <Icon size={ICON_SIZE.MD} aria-hidden="true" />
                <span className="dev-rail__label">{label}</span>
                {dot && <span className="dev-rail__dot" aria-hidden="true" />}
              </button>
            );
          })}
        </div>
        <button
          type="button"
          className="dev-icon-btn dev-rail__power"
          aria-label="Turn the simulation off"
          title="Turn the simulation off"
          disabled={!active || dev.busy}
          onClick={dev.reset}
        >
          <Power size={ICON_SIZE.MD} />
        </button>
      </nav>

      <div className="dev-pane">
        <header className="dev-pane__head">
          <div className="dev-pane__heading">
            <h2 className="dev-pane__title">{current.title}</h2>
            <p className={`dev-pane__status${active || updateSimulated ? ' is-active' : ''}`} role="status">
              <span className="dev-pane__dot" aria-hidden="true" />
              <span className="dev-pane__status-text">{devStatusText(active, preset?.label ?? null, updateSimulated)}</span>
            </p>
          </div>
          <button type="button" className="dev-icon-btn" aria-label={`Close ${TITLE.toLowerCase()}`} title="Close" onClick={onClose}>
            <X size={ICON_SIZE.MD} />
          </button>
        </header>

        <div
          key={current.id}
          id={PANEL_ID}
          role="tabpanel"
          aria-labelledby={tabId(current.id)}
          tabIndex={0}
          className="dev-pane__body"
        >
          {content()}
        </div>

        <footer className="dev-pane__foot">
          <Keyboard size={ICON_SIZE.SM} aria-hidden="true" />
          <span>
            <kbd className="dev-kbd">{shortcut}</kbd> toggles
          </span>
          <span aria-hidden="true">·</span>
          <span>
            <kbd className="dev-kbd">Esc</kbd> closes
          </span>
          <span className="dev-pane__foot-end">English only</span>
        </footer>
      </div>
    </div>
  );
}
