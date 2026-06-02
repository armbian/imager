import {
  Download,
  HardDrive,
  CircleCheck,
  CircleX,
  Check,
  Archive,
  Shield,
  ShieldCheck,
  Cpu,
  Layers,
  FolderOpen,
} from 'lucide-react';
import { UI } from '../../config';

export type FlashStage =
  | 'authorizing'
  | 'downloading'
  | 'verifying_sha'
  | 'decompressing'
  | 'extracting'
  | 'qdl_sahara'
  | 'qdl_firehose'
  | 'flashing'
  | 'verifying'
  | 'complete'
  | 'error';

interface FlashStageIconProps {
  stage: FlashStage;
  size?: number;
}

export function FlashStageIcon({ stage, size = UI.ICON_SIZE.FLASH_STAGE }: FlashStageIconProps) {
  switch (stage) {
    case 'authorizing':
      return <Shield size={size} className="stage-icon authorizing" />;
    case 'downloading':
      return <Download size={size} className="stage-icon downloading" />;
    case 'verifying_sha':
      return <ShieldCheck size={size} className="stage-icon verifying-sha" />;
    case 'decompressing':
      return <Archive size={size} className="stage-icon decompressing" />;
    case 'extracting':
      return <FolderOpen size={size} className="stage-icon decompressing" />;
    case 'qdl_sahara':
      return <Cpu size={size} className="stage-icon flashing" />;
    case 'qdl_firehose':
      return <Layers size={size} className="stage-icon flashing" />;
    case 'flashing':
      return <HardDrive size={size} className="stage-icon flashing" />;
    case 'verifying':
      return <Check size={size} className="stage-icon verifying" />;
    case 'complete':
      return <CircleCheck size={size} className="stage-icon complete" />;
    case 'error':
      return <CircleX size={size} className="stage-icon error" />;
  }
}

// eslint-disable-next-line react-refresh/only-export-components
export function getStageKey(stage: FlashStage): string {
  switch (stage) {
    case 'authorizing':
      return 'flash.authorizing';
    case 'downloading':
      return 'flash.downloading';
    case 'verifying_sha':
      return 'flash.verifyingSha';
    case 'decompressing':
      return 'flash.decompressing';
    case 'extracting':
      return 'flash.extracting';
    case 'qdl_sahara':
      return 'flash.qdlSahara';
    case 'qdl_firehose':
      return 'flash.qdlFirehose';
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

/** Maps a stage to its macro phase index (0=Download, 1=Prepare, 2=Write, 3=Verify, 4=all done). */
// eslint-disable-next-line react-refresh/only-export-components
export function stageToPhase(stage: FlashStage): number {
  switch (stage) {
    case 'authorizing':
    case 'downloading':
    case 'verifying_sha':
      return 0;
    case 'decompressing':
    case 'extracting':
      return 1;
    case 'qdl_sahara':
    case 'qdl_firehose':
    case 'flashing':
      return 2;
    case 'verifying':
      return 3;
    case 'complete':
      return 4;
    case 'error':
      return 0;
  }
}

/** Stages shown with an indeterminate (breathing) bar instead of a percentage. */
const INDETERMINATE_STAGES: FlashStage[] = ['decompressing', 'verifying_sha', 'extracting', 'qdl_sahara'];

/** True when a stage has no meaningful percentage and uses the indeterminate bar. */
// eslint-disable-next-line react-refresh/only-export-components
export function isIndeterminateStage(stage: FlashStage): boolean {
  return INDETERMINATE_STAGES.includes(stage);
}
