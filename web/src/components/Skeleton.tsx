// Loading placeholder block (.skeleton shimmer). Height comes from the call
// site to match the content being replaced.

export function Skeleton({ height }: { height?: number }) {
  return <div className="skeleton" style={height !== undefined ? { height } : undefined} />;
}
