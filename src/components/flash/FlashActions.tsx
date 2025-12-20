import { RotateCcw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { FlashStage } from './FlashStageIcon';

interface FlashActionsProps {
  stage: FlashStage;
  onComplete: () => void;
  onBack: () => void;
  onRetry: () => void;
  onCancel: () => void;
}

export function FlashActions({
  stage,
  onComplete,
  onBack,
  onRetry,
  onCancel,
}: FlashActionsProps) {
  const { t } = useTranslation();

  if (stage === 'complete') {
    return (
      <div className="flash-actions-inline">
        <button className="btn btn-secondary" onClick={onBack}>
          {t('flash.flashAnother')}
        </button>
        <button className="btn btn-primary" onClick={onComplete}>
          {t('flash.done')}
        </button>
      </div>
    );
  }

  if (stage === 'error') {
    return (
      <div className="flash-actions-inline">
        <button className="btn btn-secondary" onClick={onBack}>
          {t('flash.cancel')}
        </button>
        <button className="btn btn-primary" onClick={onRetry}>
          <RotateCcw size={16} />
          {t('flash.retry')}
        </button>
      </div>
    );
  }

  return (
    <div className="flash-actions-inline">
      <button className="btn btn-secondary" onClick={onCancel}>
        {t('flash.cancel')}
      </button>
    </div>
  );
}
