export const Spinner = ({ label = 'Loading' }: { label?: string }) => <span className="spinner" role="status" aria-label={label} />;

export const Skeleton = ({ h = 16, w = '100%', className = '' }: { h?: number; w?: number | string; className?: string }) => (
  <div className={`skeleton ${className}`} style={{ height: h, width: w }} aria-hidden />
);

/** Rows placeholder used while lists load. */
export const SkeletonRows = ({ rows = 5, h = 44 }: { rows?: number; h?: number }) => (
  <div className="col" role="status" aria-label="Loading">
    {Array.from({ length: rows }, (_, i) => (
      <Skeleton key={i} h={h} />
    ))}
  </div>
);
