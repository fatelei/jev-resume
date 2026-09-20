//! 结果表：9 列，与旧版 GPUI 表格一致。

import type { RowsState } from "../rows";
import { STATUS_TEXT, seniorityDisplay } from "../rows";

interface ResultsTableProps {
  state: RowsState;
  seniorityLabels: Readonly<Record<string, string>>;
}

export function ResultsTable({ state, seniorityLabels }: ResultsTableProps) {
  const rows = state.order.map((p) => state.byPath[p]);
  return (
    <div className="table-wrap">
      <table className="results">
        <thead>
          <tr>
            <th>文件名</th>
            <th>状态</th>
            <th>岗位分类</th>
            <th>资历级别</th>
            <th className="num">强度</th>
            <th className="num">注水</th>
            <th className="num">耗时</th>
            <th>缓存</th>
            <th>备注</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.path}>
              <td className="ellipsis" title={row.path}>{row.file_name}</td>
              <td>
                <span className={`status ${row.state}`}>{STATUS_TEXT[row.state]}</span>
              </td>
              <td>{row.judgment?.role_category ?? "-"}</td>
              <td>{seniorityDisplay(row.judgment?.seniority ?? null, seniorityLabels)}</td>
              <td className="num">{fmt(row.judgment?.strength, 1)}</td>
              <td className="num">{fmt(row.judgment?.inflation, 2)}</td>
              <td className="num">{row.elapsed_ms != null ? `${row.elapsed_ms}ms` : "-"}</td>
              <td>{row.cached ? "是" : "-"}</td>
              <td className="ellipsis note" title={row.error ?? ""}>{row.error ?? ""}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function fmt(v: number | undefined | null, digits: number): string {
  return typeof v === "number" ? v.toFixed(digits) : "-";
}
