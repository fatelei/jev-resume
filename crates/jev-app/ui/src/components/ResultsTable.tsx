//! 结果表：9 列（对齐旧版 GPUI 表格），带分页（每页 100 行）。

import { useEffect, useState } from "react";

import type { Row, RowsState } from "../rows";
import { STATUS_TEXT, seniorityDisplay } from "../rows";

const PAGE_SIZE = 100;

interface ResultsTableProps {
  state: RowsState;
  /** 已经过搜索/状态筛选的行（保持插入序） */
  rows: Row[];
  /** 筛选条件变化时重置回第一页 */
  filterKey: string;
  seniorityLabels: Readonly<Record<string, string>>;
}

export function ResultsTable({ rows, filterKey, seniorityLabels }: ResultsTableProps) {
  const [page, setPage] = useState(0);
  const pageCount = Math.max(1, Math.ceil(rows.length / PAGE_SIZE));
  const safePage = Math.min(page, pageCount - 1);

  // 筛选/搜索/清空后回到第一页
  useEffect(() => {
    setPage(0);
  }, [filterKey]);

  const slice = rows.slice(safePage * PAGE_SIZE, (safePage + 1) * PAGE_SIZE);

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
          {slice.map((row) => (
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
          {slice.length === 0 && (
            <tr>
              <td colSpan={9} className="no-match">没有匹配的行</td>
            </tr>
          )}
        </tbody>
      </table>
      {rows.length > PAGE_SIZE && (
        <div className="pager">
          <span>
            共 {rows.length} 行 · 第 {safePage + 1}/{pageCount} 页
          </span>
          <button className="btn tiny" disabled={safePage === 0} onClick={() => setPage(safePage - 1)}>
            上一页
          </button>
          <button
            className="btn tiny"
            disabled={safePage >= pageCount - 1}
            onClick={() => setPage(safePage + 1)}
          >
            下一页
          </button>
        </div>
      )}
    </div>
  );
}

function fmt(v: number | undefined | null, digits: number): string {
  return typeof v === "number" ? v.toFixed(digits) : "-";
}
