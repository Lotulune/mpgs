// Topbar status chips: offline, demo data, pending feedback sync.

import { Chip } from "../../components/Chip";

export function StatusChips({
  online,
  demoMode,
  pendingCount,
}: {
  online: boolean;
  demoMode: boolean;
  pendingCount: number;
}) {
  if (online && !demoMode && pendingCount === 0) return null;
  return (
    <span className="status-chips">
      {!online && <Chip tone="danger">离线</Chip>}
      {demoMode && <Chip tone="warn">演示数据</Chip>}
      {pendingCount > 0 && <Chip tone="warn">{pendingCount} 条待同步</Chip>}
    </span>
  );
}
