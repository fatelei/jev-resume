//! 工具栏：导入（文件/文件夹）、取消、导出、清空、搜索/筛选、设置、无 Key 提示。

import type { StatusFilter } from "../rows";
import { STATUS_FILTER_TEXT } from "../rows";

interface ToolbarProps {
  running: boolean;
  canExport: boolean;
  hasApiKey: boolean;
  configPath: string | null;
  hasRows: boolean;
  query: string;
  statusFilter: StatusFilter;
  onPickFiles: () => void;
  onPickFolder: () => void;
  onCancel: () => void;
  onExport: () => void;
  onOpenSettings: () => void;
  onQueryChange: (q: string) => void;
  onStatusFilter: (s: StatusFilter) => void;
  onClear: () => void;
}

export function Toolbar(props: ToolbarProps) {
  return (
    <div className="toolbar">
      <button className="btn primary" onClick={props.onPickFiles} disabled={props.running}>
        导入文件
      </button>
      <button className="btn" onClick={props.onPickFolder} disabled={props.running}>
        导入文件夹
      </button>
      <button className="btn danger" onClick={props.onCancel} disabled={!props.running}>
        取消
      </button>
      <button className="btn" onClick={props.onExport} disabled={!props.canExport}>
        导出
      </button>
      <button className="btn" onClick={props.onClear} disabled={props.running || !props.hasRows}>
        清空
      </button>
      <button
        className={`btn ${props.hasApiKey ? "" : "warn"}`}
        onClick={props.onOpenSettings}
        title="配置 API Key"
      >
        设置{props.hasApiKey ? "" : " (未配置 Key)"}
      </button>
      <input
        className="search"
        placeholder="搜索文件名…"
        value={props.query}
        onChange={(e) => props.onQueryChange(e.target.value)}
      />
      <select
        className="filter"
        value={props.statusFilter}
        onChange={(e) => props.onStatusFilter(e.target.value as StatusFilter)}
      >
        {(Object.keys(STATUS_FILTER_TEXT) as StatusFilter[]).map((s) => (
          <option key={s} value={s}>
            {STATUS_FILTER_TEXT[s]}
          </option>
        ))}
      </select>
      <div className="toolbar-hint">PDF / DOCX / TXT，结果本地缓存</div>
      {!props.hasApiKey && (
        <button
          className="key-warning"
          onClick={props.onOpenSettings}
          title={props.configPath ?? ""}
        >
          未配置 API Key，点击设置
        </button>
      )}
    </div>
  );
}
