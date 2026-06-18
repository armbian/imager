import { Check } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import armbianLogoWhite from '../../assets/armbian-logo-white.png';
import armbianLogoBlack from '../../assets/armbian-logo-black.png';
import type { BoardInfo, ImageInfo, BlockDevice, SelectionStep, Manufacturer } from '../../types';
import { isEdlImage } from '../../types';
import { UpdateModal } from '../shared';
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
}: HeaderProps) {
  const { t } = useTranslation();
  const isCustomImage = selectedImage?.is_custom;

  // Detected-board images show all 4 steps; generic .img files show 2.
  const hasDetectedBoard = selectedBoard && selectedBoard.slug !== 'custom' && selectedBoard.slug !== 'cached';
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

  function handleLogoClick() {
    if (!isFlashing && onReset) {
      onReset();
    }
  }

  // Back-navigation reopens API-driven panels (manufacturer/board/OS), so it's disabled offline:
  // there the only entry is a custom/cached image and those panels can't load without the network.
  const canNavigateSteps = !isFlashing && !!onNavigateToStep && isOnline;

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
            className={`header-left ${!isFlashing && onReset ? 'clickable' : ''}`}
            onClick={handleLogoClick}
            title={!isFlashing ? t('header.resetTooltip') : undefined}
          >
            {/* Black wordmark on light theme, white on dark; toggled via CSS to also cover 'auto'. */}
            <img src={armbianLogoBlack} alt="Armbian" className="logo-main logo-main--light" />
            <img src={armbianLogoWhite} alt="" aria-hidden="true" className="logo-main logo-main--dark" />
          </div>
        )}
        <div className="header-right">
          {/* Steps hidden on the welcome landing and the offline entry (banner already says it) */}
          {hideSteps || (!isOnline && !selectedManufacturer) ? null : (
            <div className="header-steps">
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
          {/* Settings lives top-right, freeing the sidebar */}
          {!hideSettings && <SettingsButton variant="inline" />}
        </div>
      </header>
    </>
  );
}
