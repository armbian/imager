// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useId, useState } from 'react';
import { RotateCcw, ScanSearch } from 'lucide-react';
import { DevSwitch } from './DevControls';
import { useAsyncData } from '../../hooks/useAsyncData';
import { getBoards } from '../../hooks/useTauri';
import { getArmbianBoardDetection } from '../../hooks/useSettings';
import { EVENTS, SETTINGS } from '../../config';
import { DEV_DETECTION_NOTES, DEV_SCENARIOS, devDefaultHostBoard, devHostBoardOptions } from '../../config/devScenarios';
import type { DevScenarioApi } from '../../hooks/useDevScenario';
import type { ArmbianDetectionOutcome, BoardInfo, DevScenariosStatus } from '../../types';

const { ICON_SIZE } = DEV_SCENARIOS;

interface DevHostProps {
  dev: DevScenarioApi;
  status: DevScenariosStatus;
  onRunDetection: (() => Promise<ArmbianDetectionOutcome>) | null;
}

export function DevHost({ dev, status, onRunDetection }: DevHostProps) {
  const searchId = useId();
  const boardId = useId();
  const noteId = useId();
  const host = status.scenario.armbianHost;
  const boards = useAsyncData(getBoards, []);
  const { data: mode, reload: reloadMode } = useAsyncData(getArmbianBoardDetection, []);
  const [query, setQuery] = useState('');
  const [picked, setPicked] = useState<string | null>(null);
  const [outcome, setOutcome] = useState<ArmbianDetectionOutcome | null>(null);
  const [running, setRunning] = useState(false);

  useEffect(() => {
    const onSettingsChanged = () => {
      setOutcome(null);
      void reloadMode();
    };
    window.addEventListener(EVENTS.SETTINGS_CHANGED, onSettingsChanged);
    return () => window.removeEventListener(EVENTS.SETTINGS_CHANGED, onSettingsChanged);
  }, [reloadMode]);

  const list = boards.data ?? [];
  const slug = host?.board ?? picked ?? devDefaultHostBoard(list)?.slug ?? '';
  const options = devHostBoardOptions(list, query, slug);
  const listed = list.some((b) => b.slug === slug);

  const setHost = (board: BoardInfo | null) => {
    setOutcome(null);
    return dev.patch((s) => ({ ...s, armbianHost: board ? { board: board.slug, board_name: board.name } : null }));
  };

  const pick = (next: string) => {
    setPicked(next);
    const board = list.find((b) => b.slug === next);
    if (host && board) void setHost(board);
  };

  const run = async () => {
    if (!onRunDetection) return;
    setRunning(true);
    try {
      setOutcome(await onRunDetection());
    } finally {
      setRunning(false);
      void reloadMode();
    }
  };

  const note = () => {
    if (!onRunDetection) return 'Offline: the app skips detection until it is back online, as at startup.';
    if (mode === SETTINGS.ARMBIAN_DETECTION_MODES.DISABLED) return DEV_DETECTION_NOTES[mode];
    if (outcome) return DEV_DETECTION_NOTES[outcome];
    return 'Runs the startup check now and follows Auto-detect board in Settings > Writing.';
  };

  return (
    <div className="dev-stack">
      <DevSwitch
        label="Running on Armbian"
        hint={host ? `getArmbianRelease answers ${host.board}` : 'getArmbianRelease answers null, as off Armbian'}
        checked={host !== null}
        disabled={dev.busy || (!host && !listed)}
        onChange={(on) => setHost(on ? (list.find((b) => b.slug === slug) ?? null) : null)}
      />

      <div className="dev-field">
        <label className="dev-field__label" htmlFor={searchId}>
          Board
        </label>
        <div className="dev-input">
          <input
            id={searchId}
            type="search"
            value={query}
            placeholder="Filter by name or slug"
            spellCheck={false}
            disabled={list.length === 0}
            aria-controls={boardId}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
        <select
          id={boardId}
          className="settings-select dev-select"
          aria-label="Simulated board"
          value={listed || host ? slug : ''}
          disabled={dev.busy || list.length === 0}
          onChange={(e) => pick(e.target.value)}
        >
          {host && !listed && (
            <option value={host.board}>
              {host.board_name} ({host.board})
            </option>
          )}
          {!host && list.length === 0 && <option value="">{boards.loading ? 'Loading boards…' : 'No boards'}</option>}
          {options.map((b) => (
            <option key={b.slug} value={b.slug}>
              {b.name} ({b.slug})
            </option>
          ))}
        </select>
      </div>

      {boards.error && (
        <div className="dev-row">
          <p className="dev-note">Board list not loaded: {boards.error}</p>
          <button type="button" className="dev-btn dev-btn--quiet" onClick={boards.reload}>
            <RotateCcw size={ICON_SIZE.SM} aria-hidden="true" />
            Retry
          </button>
        </div>
      )}

      <div className="dev-subblock">
        <button
          type="button"
          className="dev-btn dev-btn--block"
          disabled={!host || !onRunDetection || running || dev.busy}
          aria-describedby={noteId}
          onClick={run}
        >
          <ScanSearch size={ICON_SIZE.SM} aria-hidden="true" />
          Show detection window
        </button>
        <p className="dev-note" id={noteId} role="status">
          {note()}
        </p>
        <p className="dev-note">
          Closing the window saves Auto-detect board as a real run does: Confirm, ✕ or a click outside set Silent, Cancel
          or Esc sets Disabled.
        </p>
      </div>
    </div>
  );
}
