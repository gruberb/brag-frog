import { format, parseISO } from 'date-fns';
import type { Trends } from '@/types/api/models';
import { weekTotal } from '../utils/weeks';

type Week = Trends['weekly'][number];
// Colours come from the design tokens only.
const SERIES = [
  { key: 'code', label: 'Code', color: 'var(--color-primary)' },
  { key: 'review', label: 'Review', color: 'var(--color-blue-30)' },
  { key: 'meeting', label: 'Meetings', color: 'var(--color-purple-30)' },
  { key: 'other', label: 'Other', color: 'var(--color-orange-40)' },
] as const;

const W = 40;
const H = 160;
const PAD_BOTTOM = 22;

type Props = { weeks: Week[]; onSelectWeek: (weekStart: string) => void };

/** Hand-written SVG stacked bars (no chart library). Each bar is a button that opens that week in the logbook. */
export function StackedBars({ weeks, onSelectWeek }: Props) {
  const max = Math.max(1, ...weeks.map(weekTotal));
  const step = Math.ceil(weeks.length / 8);
  return (
    <div>
      <svg viewBox={`0 0 ${weeks.length * W} ${H + PAD_BOTTOM}`} width="100%" height={H + PAD_BOTTOM} role="group" aria-label="Activity by week">
        {weeks.map((w, i) => {
          let y = H;
          const total = weekTotal(w);
          return (
            <g
              key={w.week_start}
              role="link"
              tabIndex={0}
              aria-label={`Week of ${format(parseISO(w.week_start), 'MMM d')}: ${total} entries. Open in logbook`}
              style={{ cursor: 'pointer' }}
              onClick={() => onSelectWeek(w.week_start)}
              onKeyDown={(e) => e.key === 'Enter' && onSelectWeek(w.week_start)}
            >
              <rect x={i * W} y={0} width={W} height={H} fill="transparent" />
              {SERIES.map((s) => {
                const h = (w.by_type[s.key] / max) * (H - 8);
                y -= h;
                return h > 0 ? <rect key={s.key} x={i * W + 6} y={y} width={W - 12} height={h} fill={s.color} opacity={0.8}><title>{`${s.label}: ${w.by_type[s.key]}`}</title></rect> : null;
              })}
              {i % step === 0 && (
                <text x={i * W + W / 2} y={H + 15} textAnchor="middle" fontSize="10" fill="var(--color-text-secondary)">
                  {format(parseISO(w.week_start), 'MMM d')}
                </text>
              )}
            </g>
          );
        })}
      </svg>
      <div className="legend">
        {SERIES.map((s) => <span key={s.key}><b style={{ background: s.color }} />{s.label}</span>)}
        <span className="muted">Click a bar to open that week in the logbook</span>
      </div>
    </div>
  );
}
