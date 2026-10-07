// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { Update, type DownloadEvent } from '@tauri-apps/plugin-updater';
import { DEV_SIMULATED_UPDATE } from '../../config/devScenarios';
import { sleep } from '../../utils';

type OnEvent = (progress: DownloadEvent) => void;

export class SimulatedUpdate extends Update {
  private readonly fails: boolean;
  private cancelled = false;

  constructor(currentVersion: string, fails: boolean) {
    super({
      rid: DEV_SIMULATED_UPDATE.RID,
      currentVersion,
      version: DEV_SIMULATED_UPDATE.VERSION,
      date: new Date().toISOString(),
      body: DEV_SIMULATED_UPDATE.BODY,
      rawJson: {},
    });
    this.fails = fails;
  }

  get simulatesFailure(): boolean {
    return this.fails;
  }

  override async download(onEvent?: OnEvent): Promise<void> {
    const { CHUNKS, CHUNK_BYTES, CHUNK_INTERVAL_MS, FAIL_AFTER_CHUNKS, ERROR } = DEV_SIMULATED_UPDATE;
    onEvent?.({ event: 'Started', data: { contentLength: CHUNKS * CHUNK_BYTES } });
    for (let chunk = 0; chunk < CHUNKS; chunk++) {
      await sleep(CHUNK_INTERVAL_MS);
      if (this.cancelled) return;
      if (this.fails && chunk === FAIL_AFTER_CHUNKS) throw new Error(ERROR);
      onEvent?.({ event: 'Progress', data: { chunkLength: CHUNK_BYTES } });
    }
    onEvent?.({ event: 'Finished' });
  }

  override async install(): Promise<void> {}

  override async downloadAndInstall(onEvent?: OnEvent): Promise<void> {
    await this.download(onEvent);
    await this.install();
  }

  override async close(): Promise<void> {
    this.cancelled = true;
  }
}
