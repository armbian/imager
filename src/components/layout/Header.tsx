import { Check, WifiOff } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import armbianLogo from '../../assets/armbian-logo.png';
import type { BoardInfo, ImageInfo, BlockDevice, SelectionStep, Manufacturer } from '../../types';
import { UpdateModal, MotdTip } from '../shared';

interface HeaderProps {
  selectedManufacturer?: Manufacturer | null;
  selectedBoard?: BoardInfo | null;
  selectedImage?: ImageInfo | null;
  selectedDevice?: BlockDevice | null;
  onReset?: () => void;
  onNavigateToStep?: (step: SelectionStep) => void;
  isFlashing?: boolean;
  isOnline?: boolean;
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
}: HeaderProps) {
  const { t } = useTranslation();
  const isCustomImage = selectedImage?.is_custom;

  // Custom/cached images with a detected Armbian board show all 4 steps.
  // Only truly generic images (non-Armbian .img files) show 2 steps.
  const hasDetectedBoard = selectedBoard && selectedBoard.slug !== 'custom' && selectedBoard.slug !== 'cached';
  const isGenericCustom = isCustomImage && !hasDetectedBoard;
  const steps = isGenericCustom
    ? [
        { key: 'image' as SelectionStep, label: t('header.stepImage'), completed: !!selectedImage },
        { key: 'device' as SelectionStep, label: t('header.stepStorage'), completed: !!selectedDevice },
      ]
    : [
        { key: 'manufacturer' as SelectionStep, label: t('header.stepManufacturer'), completed: !!selectedManufacturer },
        { key: 'board' as SelectionStep, label: t('header.stepBoard'), completed: !!selectedBoard },
        { key: 'image' as SelectionStep, label: t('header.stepOs'), completed: !!selectedImage },
        { key: 'device' as SelectionStep, label: t('header.stepStorage'), completed: !!selectedDevice },
      ];

  function handleLogoClick() {
    if (!isFlashing && onReset) {
      onReset();
    }
  }

  function handleStepClick(step: SelectionStep, completed: boolean) {
    // Only allow clicking on completed steps, and not during flashing
    if (!isFlashing && completed && onNavigateToStep) {
      onNavigateToStep(step);
    }
  }

  return (
    <>
      <UpdateModal />
      <header className="header">
        <div className="header-left">
          <img
            src={armbianLogo}
            alt="Armbian"
            className={`logo-main ${!isFlashing && onReset ? 'clickable' : ''}`}
            onClick={handleLogoClick}
            title={!isFlashing ? t('header.resetTooltip') : undefined}
          />
        </div>
        {/* Offline with no selections: show compact offline badge instead of steps */}
        {(!isOnline && !selectedManufacturer) ? (
          <div className="header-steps">
            <div className="header-step header-offline-badge">
              <WifiOff size={14} />
              <span className="header-step-label">{t('common.offline')}</span>
            </div>
          </div>
        ) : (
          <div className="header-steps">
            {steps.map((step, index) => (
              <div
                key={step.key}
                className={`header-step ${step.completed ? 'completed' : ''} ${!isFlashing && step.completed && onNavigateToStep ? 'clickable' : ''}`}
                onClick={() => handleStepClick(step.key, step.completed)}
                title={!isFlashing && step.completed ? t('header.stepTooltip', { step: step.label }) : undefined}
              >
                <span className="header-step-indicator">
                  {step.completed ? <Check size={14} /> : (index + 1)}
                </span>
                <span className="header-step-label">{step.label}</span>
              </div>
            ))}
          </div>
        )}
      </header>
      {!isOnline && (
        <div className="offline-banner">
          <WifiOff size={14} />
          <span>{t('home.offlineBanner')}</span>
        </div>
      )}
      {isOnline && <MotdTip />}
    </>
  );
}
