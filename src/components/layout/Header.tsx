import { Check } from 'lucide-react';
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
}

export function Header({
  selectedManufacturer,
  selectedBoard,
  selectedImage,
  selectedDevice,
  onReset,
  onNavigateToStep,
  isFlashing,
}: HeaderProps) {
  const { t } = useTranslation();
  const isCustomImage = selectedImage?.is_custom;

  // For custom images, show different steps
  const steps = isCustomImage
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
      </header>
      <MotdTip />
    </>
  );
}
