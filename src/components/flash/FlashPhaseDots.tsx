import { stagePhase, PHASE_ORDER, type FlashPhase, type FlashStage } from './FlashStageIcon';

/** Active dot index for the current stage within the planned phases. */
function activeIndex(stage: FlashStage, phases: FlashPhase[]): number {
  if (stage === 'complete') return phases.length;
  const phase = stagePhase(stage);
  if (!phase) return 0;
  const idx = phases.indexOf(phase);
  if (idx !== -1) return idx;
  // Stage belongs to a phase we didn't plan: treat earlier-ranked planned phases as done.
  const rank = PHASE_ORDER.indexOf(phase);
  return phases.filter((p) => PHASE_ORDER.indexOf(p) < rank).length;
}

/** Progress dots for the phases that will actually run; the current one elongates into a pill. */
export function FlashPhaseDots({ stage, phases }: { stage: FlashStage; phases: FlashPhase[] }) {
  if (phases.length < 2) return null;
  const active = activeIndex(stage, phases);
  return (
    <div className="flash-dots" role="presentation" aria-hidden="true">
      {phases.map((phase, i) => {
        const done = i < active;
        const current = i === active;
        return (
          <span
            key={phase}
            className={`flash-dot${done ? ' is-done' : ''}${current ? ' is-current' : ''}`}
          />
        );
      })}
    </div>
  );
}
