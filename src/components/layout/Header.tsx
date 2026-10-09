// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import type { Ref } from 'react';
import { ArrowLeft, Check } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import armbianLogoWhite from '../../assets/armbian-logo-white.png';
import armbianLogoBlack from '../../assets/armbian-logo-black.png';
import type { BoardInfo, ImageInfo, BlockDevice, SelectionStep, Manufacturer } from '../../types';
import { isEdlImage } from '../../types';
import { UpdateModal } from '../shared';
import { isDetectedBoard } from '../../utils';
import { SettingsButton } from '../settings';

interface HeaderProps {
  selectedManufacturer?: Manufacturer | null;
  selectedBoard?: BoardInfo | null;
  selectedImage?: ImageInfo | null;
  selectedDevice?: BlockDevice | null;
  onReset?: () => void;
  onNavigateToStep?: (step: SelectionStep) => void;
  isFlashing?: boolean;
  isOnline?: boolean;
  /** Hide the step progress pill (e.g. on the welcome landing). */
  hideSteps?: boolean;
  /** Hide the settings gear (e.g. on the welcome landing). */
  hideSettings?: boolean;
  /** Hide the wordmark (e.g. on the welcome landing, where the hero already brands it). */
  hideLogo?: boolean;
  /** True briefly after leaving the welcome screen to drive the one-shot entrance animation. */
  entering?: boolean;
  /** The settings page shows: the step capsule gives way to the "Back to flashing" pill. */
  settingsOpen?: boolean;
  onOpenSettings?: () => void;
  onCloseSettings?: () => void;
  settingsButtonRef?: Ref<HTMLButtonElement>;
}

export function Header({
  selectedManufacturer,
  selectedBoard,
  selectedImage,
  selectedDevice,
  onReset,
  onNavigateToStep,
  isFlashing,
  isOnline = true,
  hideSteps = false,
  hideSettings = false,
  hideLogo = false,
  entering = false,
  settingsOpen = false,
  onOpenSettings,
  onCloseSettings,
  settingsButtonRef,
}: HeaderProps) {
  const { t } = useTranslation();
  const isCustomImage = selectedImage?.is_custom;

  // Detected-board images show all 4 steps; generic .img files show 2.
  const hasDetectedBoard = isDetectedBoard(selectedBoard);
  const isGenericCustom = isCustomImage && !hasDetectedBoard;
  // EDL targets are USB devices in download mode, not storage drives.
  const isEdl = !!selectedImage && isEdlImage(selectedImage);
  const targetLabel = t(isEdl ? 'header.stepDevice' : 'header.stepStorage');
  const steps = isGenericCustom
    ? [
        { key: 'image' as SelectionStep, label: t('header.stepImage'), completed: !!selectedImage },
        { key: 'device' as SelectionStep, label: targetLabel, completed: !!selectedDevice },
      ]
    : [
        { key: 'manufacturer' as SelectionStep, label: t('header.stepManufacturer'), completed: !!selectedManufacturer },
        { key: 'board' as SelectionStep, label: t('header.stepBoard'), completed: !!selectedBoard },
        { key: 'image' as SelectionStep, label: t('header.stepOs'), completed: !!selectedImage },
        { key: 'device' as SelectionStep, label: targetLabel, completed: !!selectedDevice },
      ];

  const canReset = !isFlashing && !settingsOpen && !!onReset;

  function handleLogoClick() {
    if (canReset) onReset?.();
  }

  // Back-navigation reopens API-driven panels (manufacturer/board/OS), so it's disabled offline:
  // there the only entry is a custom/cached image and those panels can't load without the network.
  const canNavigateSteps = !isFlashing && !settingsOpen && !!onNavigateToStep && isOnline;
  const showSteps = !hideSteps && (isOnline || !!selectedManufacturer);

  function handleStepClick(step: SelectionStep, completed: boolean) {
    if (canNavigateSteps && completed) {
      onNavigateToStep!(step);
    }
  }

  return (
    <>
      <UpdateModal />
      <header className={`header ${entering ? 'is-entering' : ''}`} data-tauri-drag-region>
        {/* Wordmark hidden on the welcome landing, where the hero already brands the app. */}
        {hideLogo ? (
          <div className="header-left" />
        ) : (
          <div
            className={`header-left ${canReset ? 'clickable' : ''}`}
            onClick={handleLogoClick}
            title={canReset ? t('header.resetTooltip') : undefined}
          >
            {/* Black wordmark on light theme, white on dark; toggled via CSS to also cover 'auto'. */}
            <img src={armbianLogoBlack} alt="Armbian" className="logo-main logo-main--light" />
            <img src={armbianLogoWhite} alt="" aria-hidden="true" className="logo-main logo-main--dark" />
          </div>
        )}
        <div className="header-right">
          <div className={`header-controls${settingsOpen ? ' is-settings' : ''}`}>
            {/* Steps hidden on the welcome landing and the offline entry (banner already says it) */}
            {showSteps && (
              <div className="header-steps" aria-hidden={settingsOpen || undefined} inert={settingsOpen || undefined}>
                {steps.map((step, index) => (
                  <div
                    key={step.key}
                    className={`header-step ${step.completed ? 'completed' : ''} ${canNavigateSteps && step.completed ? 'clickable' : ''}`}
                    onClick={() => handleStepClick(step.key, step.completed)}
                    title={canNavigateSteps && step.completed ? t('header.stepTooltip', { step: step.label }) : undefined}
                  >
                    <span className="header-step-indicator">
                      {step.completed ? <Check size={14} /> : (index + 1)}
                    </span>
                    <span className="header-step-label">{step.label}</span>
                  </div>
                ))}
              </div>
            )}
            {!hideSettings && (
              <button
                type="button"
                className="header-back"
                onClick={onCloseSettings}
                aria-hidden={!settingsOpen || undefined}
                inert={!settingsOpen || undefined}
              >
                <ArrowLeft size={16} aria-hidden="true" />
                {t('settings.backToFlashing')}
              </button>
            )}
          </div>
          {!hideSettings && (
            <SettingsButton
              ref={settingsButtonRef}
              active={settingsOpen}
              onClick={() => (settingsOpen ? onCloseSettings?.() : onOpenSettings?.())}
            />
          )}
        </div>
      </header>
    </>
  );
}
