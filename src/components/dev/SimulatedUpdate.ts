// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { Update, type DownloadEvent } from '@tauri-apps/plugin-updater';
import { DEV_SIMULATED_UPDATE, DEV_UPDATE_MODE, type DevUpdateMode } from '../../config/devScenarios';
import { sleep } from '../../utils';

type OnEvent = (progress: DownloadEvent) => void;

export class SimulatedUpdate extends Update {
  private readonly simMode: DevUpdateMode;
  private cancelled = false;
  private downloaded = false;

  constructor(currentVersion: string, mode: DevUpdateMode) {
    super({
      rid: DEV_SIMULATED_UPDATE.RID,
      currentVersion,
      version: DEV_SIMULATED_UPDATE.VERSION,
      date: new Date().toISOString(),
      body: DEV_SIMULATED_UPDATE.BODY,
      rawJson: {},
    });
    this.simMode = mode;
  }

  get mode(): DevUpdateMode {
    return this.simMode;
  }

  override async download(onEvent?: OnEvent): Promise<void> {
    const { CHUNKS, CHUNK_BYTES, CHUNK_INTERVAL_MS, FAIL_AFTER_CHUNKS, ERROR } = DEV_SIMULATED_UPDATE;
    const sized = this.simMode !== DEV_UPDATE_MODE.UNKNOWN_SIZE;
    onEvent?.({ event: 'Started', data: sized ? { contentLength: CHUNKS * CHUNK_BYTES } : {} });
    for (let chunk = 0; chunk < CHUNKS; chunk++) {
      await sleep(CHUNK_INTERVAL_MS);
      if (this.cancelled) return;
      if (this.simMode === DEV_UPDATE_MODE.FAIL_DOWNLOAD && chunk === FAIL_AFTER_CHUNKS) throw new Error(ERROR);
      onEvent?.({ event: 'Progress', data: { chunkLength: CHUNK_BYTES } });
    }
    onEvent?.({ event: 'Finished' });
    this.downloaded = true;
  }

  // Mirrors the plugin: a failed install keeps the bytes, a good one consumes them
  override async install(): Promise<void> {
    if (!this.downloaded) throw new Error(DEV_SIMULATED_UPDATE.INSTALL_BEFORE_DOWNLOAD);
    await sleep(DEV_SIMULATED_UPDATE.INSTALL_DELAY_MS);
    if (this.cancelled) return;
    if (this.simMode === DEV_UPDATE_MODE.FAIL_INSTALL) throw new Error(DEV_SIMULATED_UPDATE.INSTALL_ERROR);
    this.downloaded = false;
  }

  override async downloadAndInstall(onEvent?: OnEvent): Promise<void> {
    await this.download(onEvent);
    await this.install();
  }

  override async close(): Promise<void> {
    this.cancelled = true;
  }
}
