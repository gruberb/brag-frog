import type { Cycle } from '@/types/api/models';

export function ReadOnlyBanner({ cycle, onBack }: { cycle: Cycle; onBack: () => void }) {
  return (
    <div className="banner" role="status">
      <span>
        You are viewing <b>{cycle.name}</b>, a past cycle. It is read-only.
      </span>
      <button type="button" className="btn sm" onClick={onBack}>
        Back to active cycle
      </button>
    </div>
  );
}
