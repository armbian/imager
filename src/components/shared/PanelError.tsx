// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useId, useRef, useState, type CSSProperties } from 'react';
import { RotateCcw, Loader2, Upload } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { UI } from '../../config';
import { HERO_ART, PANEL_ERROR_HERO } from '../../config/heroArt';
import { useEntrancePlayback } from '../../hooks/useEntrancePlayback';
import { logWarn } from '../../hooks/useTauri';
import { useLogUpload } from '../../hooks/useLogUpload';
import { useToasts } from '../../hooks/useToasts';
import { classifyLoadError, qdlErrorKey, type LoadErrorKind } from '../../utils/errorUtils';
import { HeroArt } from '../settings/HeroArt';
import { LogShareModal } from './LogShareModal';

type PanelErrorSource = 'catalog' | 'devices' | 'edl';

interface PanelErrorCopy {
  titleKey: string;
  hintKey: string;
}

const CATALOG_COPY: Record<LoadErrorKind, PanelErrorCopy> = {
  server: { titleKey: 'errorDisplay.serverTitle', hintKey: 'errorDisplay.serverHint' },
  network: { titleKey: 'errorDisplay.networkTitle', hintKey: 'errorDisplay.networkHint' },
  other: { titleKey: 'errorDisplay.otherTitle', hintKey: 'errorDisplay.otherHint' },
};

const DEVICE_COPY: Record<Exclude<PanelErrorSource, 'catalog'>, PanelErrorCopy> = {
  devices: { titleKey: 'errorDisplay.devicesTitle', hintKey: 'errorDisplay.devicesHint' },
  edl: { titleKey: 'errorDisplay.edlDevicesTitle', hintKey: 'errorDisplay.edlDevicesHint' },
};

// The backend logs the raw failure of every list command except get_qdl_devices.
const RAW_UNLOGGED: readonly PanelErrorSource[] = ['edl'];

interface PanelErrorProps {
  error: string;
  onRetry: (byPointer: boolean) => void;
  retrying: boolean;
  source?: PanelErrorSource;
}

/** Load failure of a selection panel; the scene lights up while a retry runs. */
export function PanelError({ error, onRetry, retrying, source = 'catalog' }: PanelErrorProps) {
  const { t } = useTranslation();
  const { showError } = useToasts();
  const { upload, uploading, url: pasteUrl, qr, error: uploadError, open } = useLogUpload({ withQr: true });
  const { ref: artRef, playing } = useEntrancePlayback(UI.SETTINGS_HERO.PLAY_THRESHOLD);
  const [shareOpen, setShareOpen] = useState(false);
  const loggedRef = useRef(false);
  const pointerRef = useRef(false);
  const titleId = useId();

  const catalogKind = classifyLoadError(error);
  const kind = source === 'catalog' ? catalogKind : source;
  const copy = source === 'catalog' ? CATALOG_COPY[catalogKind] : DEVICE_COPY[source];
  const hero = PANEL_ERROR_HERO[source === 'catalog' ? 'catalog' : 'devices'];
  const qdlKey = source === 'edl' ? qdlErrorKey(error) : undefined;

  useEffect(() => {
    if (uploadError) showError(uploadError);
  }, [uploadError, showError]);

  useEffect(() => {
    if (retrying) {
      loggedRef.current = false;
      return;
    }
    if (loggedRef.current) return;
    loggedRef.current = true;
    const raw = RAW_UNLOGGED.includes(source) ? `: ${error}` : '';
    logWarn('panel-error', `Panel load failed (${kind})${raw}`);
  }, [retrying, source, kind, error]);

  async function handleUpload() {
    if (await upload()) setShareOpen(true);
  }

  return (
    <section
      className={UI.PANEL_ERROR_CLASS}
      aria-labelledby={titleId}
      onPointerDownCapture={() => (pointerRef.current = true)}
      onKeyDownCapture={() => (pointerRef.current = false)}
    >
      <div className="panel-fail__column">
        <div
          ref={artRef}
          className="panel-fail__art"
          style={{ '--hero-ratio': HERO_ART[hero.id].canvas.w / HERO_ART[hero.id].canvas.h } as CSSProperties}
        >
          <HeroArt hero={hero.id} playing={playing} state={retrying ? hero.trying : hero.failed} />
        </div>
        <div className="panel-fail__message" role="alert">
          <h2 id={titleId} className="panel-fail__title">
            {t(copy.titleKey)}
          </h2>
          <p className="panel-fail__hint">{t(qdlKey ?? copy.hintKey)}</p>
        </div>
        <div className="panel-fail__actions">
          <button
            type="button"
            className="btn btn-primary btn-pill"
            onClick={() => !retrying && onRetry(pointerRef.current)}
            aria-disabled={retrying}
            aria-busy={retrying}
          >
            {retrying ? (
              <Loader2 size={UI.ICON_SIZE.BUTTON} className="spinning" />
            ) : (
              <RotateCcw size={UI.ICON_SIZE.BUTTON} />
            )}
            {retrying ? t('errorDisplay.retrying') : t('errorDisplay.retry')}
          </button>
          <button
            type="button"
            className="btn btn-secondary btn-pill"
            onClick={handleUpload}
            disabled={uploading}
            aria-busy={uploading}
          >
            {uploading ? (
              <Loader2 size={UI.ICON_SIZE.BUTTON} className="spinning" />
            ) : (
              <Upload size={UI.ICON_SIZE.BUTTON} />
            )}
            {uploading ? t('errorDisplay.uploadingLogs') : t('errorDisplay.uploadLogsForSupport')}
          </button>
        </div>
      </div>

      <LogShareModal isOpen={shareOpen} onClose={() => setShareOpen(false)} url={pasteUrl} qr={qr} onOpen={open} />
    </section>
  );
}
