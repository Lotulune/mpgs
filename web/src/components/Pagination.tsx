// Page-based pagination bar: meta text plus a numbered window of pages.
// Moved out of FeedScreen so any list screen can page the same way.

import { Button } from "./Button";

export function Pagination({
  page,
  totalPages,
  total,
  loading,
  onPage,
}: {
  page: number;
  totalPages: number;
  total: number;
  loading: boolean;
  onPage: (page: number) => void;
}) {
  if (totalPages <= 1) {
    return total > 0 ? (
      <div className="pagination" aria-label="分页">
        <span className="pagination-meta">共 {total} 款</span>
      </div>
    ) : null;
  }

  const windowSize = 5;
  let start = Math.max(1, page - Math.floor(windowSize / 2));
  let end = Math.min(totalPages, start + windowSize - 1);
  start = Math.max(1, end - windowSize + 1);
  const pages: number[] = [];
  for (let p = start; p <= end; p += 1) pages.push(p);

  return (
    <div className="pagination" aria-label="分页">
      <span className="pagination-meta">
        第 {page}/{totalPages} 页 · 共 {total} 款
      </span>
      <div className="pagination-controls">
        <Button size="small" disabled={loading || page <= 1} onClick={() => onPage(page - 1)}>
          上一页
        </Button>
        {start > 1 && (
          <>
            <Button size="small" variant="ghost" disabled={loading} onClick={() => onPage(1)}>
              1
            </Button>
            {start > 2 && <span className="pagination-ellipsis">…</span>}
          </>
        )}
        {pages.map((p) => (
          <Button
            key={p}
            size="small"
            variant={p === page ? "primary" : "ghost"}
            disabled={loading || p === page}
            aria-current={p === page ? "page" : undefined}
            onClick={() => onPage(p)}
          >
            {p}
          </Button>
        ))}
        {end < totalPages && (
          <>
            {end < totalPages - 1 && <span className="pagination-ellipsis">…</span>}
            <Button size="small" variant="ghost" disabled={loading} onClick={() => onPage(totalPages)}>
              {totalPages}
            </Button>
          </>
        )}
        <Button size="small" disabled={loading || page >= totalPages} onClick={() => onPage(page + 1)}>
          下一页
        </Button>
      </div>
    </div>
  );
}
