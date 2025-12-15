import {
  Download,
  HardDrive,
  CheckCircle,
  XCircle,
  Check,
  Archive,
  Shield,
} from 'lucide-react';

export type FlashStage =
  | 'authorizing'
  | 'downloading'
  | 'decompressing'
  | 'flashing'
  | 'verifying'
  | 'complete'
  | 'error';

interface FlashStageIconProps {
  stage: FlashStage;
  size?: number;
}

export function FlashStageIcon({ stage, size = 32 }: FlashStageIconProps) {
  switch (stage) {
    case 'authorizing':
      return <Shield size={size} className="stage-icon authorizing" />;
    case 'downloading':
      return <Download size={size} className="stage-icon downloading" />;
    case 'decompressing':
      return <Archive size={size} className="stage-icon decompressing" />;
    case 'flashing':
      return <HardDrive size={size} className="stage-icon flashing" />;
    case 'verifying':
      return <Check size={size} className="stage-icon verifying" />;
    case 'complete':
      return <CheckCircle size={size} className="stage-icon complete" />;
    case 'error':
      return <XCircle size={size} className="stage-icon error" />;
  }
}

// eslint-disable-next-line react-refresh/only-export-components
export function getStageKey(stage: FlashStage): string {
  switch (stage) {
    case 'authorizing':
      return 'flash.authorizing';
    case 'downloading':
      return 'flash.downloading';
    case 'decompressing':
      return 'flash.decompressing';
    case 'flashing':
      return 'flash.writing';
    case 'verifying':
      return 'flash.verifying';
    case 'complete':
      return 'flash.complete';
    case 'error':
      return 'flash.failed';
  }
}
