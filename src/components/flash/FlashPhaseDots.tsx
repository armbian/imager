import { stageToPhase, type FlashStage } from './FlashStageIcon';

// Macro phases shown as dots: Download · Prepare · Write · Verify.
const PHASES = 4;

/** Phase dots under the progress bar; current phase elongates into a pill. */
export function FlashPhaseDots({ stage }: { stage: FlashStage }) {
  const active = stageToPhase(stage);
  return (
    <div className="flash-dots" role="presentation" aria-hidden="true">
      {Array.from({ length: PHASES }, (_, i) => {
        const done = i < active;
        const current = i === active;
        return (
          <span
            key={i}
            className={`flash-dot${done ? ' is-done' : ''}${current ? ' is-current' : ''}`}
          />
        );
      })}
    </div>
  );
}
