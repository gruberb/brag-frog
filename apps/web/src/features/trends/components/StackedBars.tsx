import { useState } from 'react';
import { addDays, format, parseISO } from 'date-fns';
import type { Trends } from '@/types/api/models';
import { CATEGORY_SERIES } from '../utils/categories';
import { weekTotal } from '../utils/weeks';

type Week = Trends['weekly'][number];

/** Fixed plot height, so the card does not grow or shrink with its neighbours or the data. */
const PLOT_HEIGHT = 200;

type Props = { weeks: Week[]; onSelectWeek: (weekStart: string) => void };

const range = (weekStart: string) => {
  const start = parseISO(weekStart);
  return `${format(start, 'MMM d')} to ${format(addDays(start, 6), 'MMM d')}`;
};

/** Stacked bars drawn with flexbox (no chart library). Hovering or focusing a bar shows its numbers; clicking opens that week in the logbook. */
export function StackedBars({ weeks, onSelectWeek }: Props) {
  const [active, setActive] = useState<number | null>(null);
  const max = Math.max(1, ...weeks.map(weekTotal));
  const step = Math.ceil(weeks.length / 8);
  const shown = active != null ? weeks[active] : null;

  return (
    <div>
      <div className="chart-info" aria-live="polite">
        {shown ? (
          <>
            <b>{range(shown.week_start)}</b>
            <span>{weekTotal(shown)} entries</span>
            {/* Only categories present that week; eight zero counts would bury the ones that matter. */}
            {CATEGORY_SERIES.filter((s) => shown.by_category[s.key] > 0).map((s) => (
              <span key={s.key} className="chart-info-item"><i style={{ background: s.color }} />{s.label} {shown.by_category[s.key]}</span>
            ))}
          </>
        ) : (
          <span className="muted">Hover a bar for details, click it to open that week in the logbook.</span>
        )}
      </div>
      <div className="chart-plot" style={{ height: PLOT_HEIGHT }} role="group" aria-label="Activity by week" onMouseLeave={() => setActive(null)}>
        {weeks.map((w, i) => {
          const total = weekTotal(w);
          return (
            <button
              key={w.week_start}
              type="button"
              className={`chart-col${active === i ? ' on' : ''}`}
              aria-label={`Week of ${format(parseISO(w.week_start), 'MMM d')}: ${total} entries. Open in logbook`}
              onClick={() => onSelectWeek(w.week_start)}
              onMouseEnter={() => setActive(i)}
              onFocus={() => setActive(i)}
              onBlur={() => setActive(null)}
            >
              <span className="chart-area">
                <span className="chart-stack" style={{ height: `${(total / max) * 100}%` }}>
                  {CATEGORY_SERIES.map((s) =>
                    w.by_category[s.key] > 0 ? <span key={s.key} style={{ flexGrow: w.by_category[s.key], background: s.color }} /> : null,
                  )}
                </span>
              </span>
              <span className="chart-label">{i % step === 0 ? format(parseISO(w.week_start), 'MMM d') : ''}</span>
            </button>
          );
        })}
      </div>
      <div className="legend">
        {CATEGORY_SERIES.map((s) => <span key={s.key}><b style={{ background: s.color }} />{s.label}</span>)}
      </div>
    </div>
  );
}
