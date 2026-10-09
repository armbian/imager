// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useCallback, useEffect, useEffectEvent, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { DevLauncher } from './DevLauncher';
import { DevDrawer } from './DevDrawer';
import { useSimulatedUpdate } from './useSimulatedUpdate';
import { useDevScenario } from '../../hooks/useDevScenario';
import { useModalExitAnimation } from '../../hooks/useModalExitAnimation';
import { useRestoreSavedAppearance } from '../../hooks/useRestoreSavedAppearance';
import { useWindowFullscreen } from '../../hooks/useWindowFullscreen';
import { logWarn } from '../../hooks/useTauri';
import { DEV_SCENARIOS, devRootPlatformClass, devShortcutLabel } from '../../config/devScenarios';
import { getErrorMessage } from '../../utils';
import type { ArmbianDetectionOutcome, CustomImageInfo } from '../../types';
import '../../styles/dev.css';

interface DevScenariosProps {
  hidden: boolean;
  isFlashing: boolean;
  isConfirming: boolean;
  onUseCustomImage: (image: CustomImageInfo) => Promise<void>;
  onResetFlow: (() => void) | null;
  onRunArmbianDetection: (() => Promise<ArmbianDetectionOutcome>) | null;
}

const SHORTCUT = devShortcutLabel();
const ROOT_CLASS = ['dev-root', devRootPlatformClass()].filter(Boolean).join(' ');

export default function DevScenarios({
  hidden,
  isFlashing,
  isConfirming,
  onUseCustomImage,
  onResetFlow,
  onRunArmbianDetection,
}: DevScenariosProps) {
  const dev = useDevScenario();
  const fullscreen = useWindowFullscreen();
  const { simulated, clear: clearSimulatedUpdate } = useSimulatedUpdate();
  const restoreSavedAppearance = useRestoreSavedAppearance();
  const launcherRef = useRef<HTMLButtonElement>(null);
  const drawerRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const inFlashFlow = isFlashing || isConfirming;
  const [prevInFlashFlow, setPrevInFlashFlow] = useState(inFlashFlow);
  const [prevHidden, setPrevHidden] = useState(hidden);

  // Collapse once when the confirm or flash screen appears; reopening stays allowed (compact)
  if (inFlashFlow !== prevInFlashFlow) {
    setPrevInFlashFlow(inFlashFlow);
    if (inFlashFlow) setOpen(false);
  }

  if (hidden !== prevHidden) {
    setPrevHidden(hidden);
    if (hidden) setOpen(false);
  }

  const endSimulation = useEffectEvent(() => {
    void dev.resetSilently();
    if (simulated) void clearSimulatedUpdate();
    restoreSavedAppearance().catch((err) =>
      logWarn('dev', `Saved appearance not restored: ${getErrorMessage(err, 'unknown error')}`)
    );
  });

  // Every session starts from welcome with the simulation off: the backend scenario outlives a reload
  useEffect(() => {
    if (hidden) endSimulation();
  }, [hidden]);

  const { isExiting, handleClose } = useModalExitAnimation({
    onClose: useCallback(() => {
      if (drawerRef.current?.contains(document.activeElement)) launcherRef.current?.focus();
      setOpen(false);
    }, []),
  });

  const toggle = useCallback(() => {
    if (open) {
      handleClose();
      return;
    }
    setOpen(true);
  }, [open, handleClose]);

  useEffect(() => {
    if (open) drawerRef.current?.focus();
  }, [open]);

  useEffect(() => {
    if (hidden) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === DEV_SCENARIOS.SHORTCUT_KEY) {
        e.preventDefault();
        toggle();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [hidden, toggle]);

  if (!dev.status || hidden) return null;

  const updateSimulated = simulated !== null;
  const active = dev.status.active || updateSimulated;

  return createPortal(
    <div id={DEV_SCENARIOS.ROOT_ID} className={ROOT_CLASS}>
      {active && <div className={`dev-frame${fullscreen ? ' is-fullscreen' : ''}`} aria-hidden="true" />}
      {open && (
        <DevDrawer
          ref={drawerRef}
          dev={dev}
          status={dev.status}
          compact={inFlashFlow}
          isExiting={isExiting}
          isFlashing={isFlashing}
          updateSimulated={updateSimulated}
          shortcut={SHORTCUT}
          onClose={handleClose}
          onUseCustomImage={onUseCustomImage}
          onResetFlow={onResetFlow}
          onRunArmbianDetection={onRunArmbianDetection}
        />
      )}
      <DevLauncher ref={launcherRef} open={open} active={active} shortcut={SHORTCUT} onToggle={toggle} />
    </div>,
    document.body
  );
}
