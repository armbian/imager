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

/** Macro phases shown as progress dots: Download · Prepare · Write · Verify. */
export type FlashPhase = 'download' | 'prepare' | 'write' | 'verify';

/** Canonical phase order. */
// eslint-disable-next-line react-refresh/only-export-components
export const PHASE_ORDER: FlashPhase[] = ['download', 'prepare', 'write', 'verify'];

/** Maps a stage to its macro phase, or null for stages without a dot (authorizing/complete/error). */
// eslint-disable-next-line react-refresh/only-export-components
export function stagePhase(stage: FlashStage): FlashPhase | null {
  switch (stage) {
    case 'downloading':
    case 'verifying_sha':
      return 'download';
    case 'decompressing':
    case 'extracting':
      return 'prepare';
    case 'qdl_sahara':
    case 'qdl_firehose':
    case 'flashing':
      return 'write';
    case 'verifying':
      return 'verify';
    case 'authorizing':
    case 'complete':
    case 'error':
      return null;
  }
}

/** Stages shown with an indeterminate (breathing) bar instead of a percentage. */
const INDETERMINATE_STAGES: FlashStage[] = ['decompressing', 'verifying_sha', 'extracting', 'qdl_sahara'];

/** True when a stage has no meaningful percentage and uses the indeterminate bar. */
// eslint-disable-next-line react-refresh/only-export-components
export function isIndeterminateStage(stage: FlashStage): boolean {
  return INDETERMINATE_STAGES.includes(stage);
}
