import { Check } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import armbianLogo from '../assets/armbian-logo.png';
import type { BoardInfo, ImageInfo, BlockDevice } from '../types';
import type { Manufacturer } from './ManufacturerModal';
import { UpdateModal } from './shared/UpdateModal';

interface HeaderProps {
  selectedManufacturer?: Manufacturer | null;
  selectedBoard?: BoardInfo | null;
  selectedImage?: ImageInfo | null;
  selectedDevice?: BlockDevice | null;
}

export function Header({
  selectedManufacturer,
  selectedBoard,
  selectedImage,
  selectedDevice,
}: HeaderProps) {
  const { t } = useTranslation();
  const isCustomImage = selectedImage?.is_custom;

  // For custom images, show different steps
  const steps = isCustomImage
    ? [
        { label: t('header.stepImage'), completed: !!selectedImage },
        { label: t('header.stepStorage'), completed: !!selectedDevice },
      ]
    : [
        { label: t('header.stepManufacturer'), completed: !!selectedManufacturer },
        { label: t('header.stepBoard'), completed: !!selectedBoard },
        { label: t('header.stepOs'), completed: !!selectedImage },
        { label: t('header.stepStorage'), completed: !!selectedDevice },
      ];

  return (
    <>
      <UpdateModal />
      <header className="header">
        <div className="header-left">
          <img src={armbianLogo} alt="Armbian" className="logo-main" />
        </div>
        <div className="header-steps">
          {steps.map((step, index) => (
            <div key={step.label} className={`header-step ${step.completed ? 'completed' : ''}`}>
              <span className="header-step-indicator">
                {step.completed ? <Check size={14} /> : (index + 1)}
              </span>
              <span className="header-step-label">{step.label}</span>
            </div>
          ))}
        </div>
      </header>
    </>
  );
}
