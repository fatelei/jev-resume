//! 状态栏：进度摘要 + 最新日志/提示。

import type { RowsState } from "../rows";
import { countByState, countCached } from "../rows";

interface StatusBarProps {
  state: RowsState;
  notice: string | null;
}

export function StatusBar({ state, notice }: StatusBarProps) {
  const done = countByState(state, "done");
  const failed = countByState(state, "failed");
  const cached = countCached(state);

  const summary =
    state.order.length === 0
      ? "拖入简历文件或文件夹开始"
      : state.running
        ? `${state.total} 个文件判定中 · 已完成 ${done} · 失败 ${failed} · 缓存 ${cached}`
        : `共 ${state.order.length} 个文件 · 完成 ${done} · 失败 ${failed} · 缓存 ${cached}`;

  const latestLog = state.logs[0] ?? null;

  return (
    <div className="statusbar">
      <span>{summary}</span>
      <span className={notice ? "notice" : "log"}>{notice ?? latestLog ?? ""}</span>
    </div>
  );
}
