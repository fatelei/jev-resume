//! 行状态与流水线事件消费（纯函数 reducer，全不可变更新）。
//!
//! 类型对齐 jev-core `pipeline.rs` 的 serde 输出：
//! PipelineEvent 无 serde(tag)，JSON 是外部标签形状 {"FileUpdated": {...}}。

export type FileState =
  | "pending"
  | "extracting"
  | "queued"
  | "judging"
  | "done"
  | "failed";

export interface FileJudgment {
  role_category: string | null;
  role_confidence: number | null;
  seniority: string | null;
  strength: number | null;
  inflation: number | null;
  model: string | null;
  input_tokens: number;
  output_tokens: number;
}

export interface FileUpdatePayload {
  path: string;
  state: FileState;
  judgment: FileJudgment | null;
  error: string | null;
  elapsed_ms: number | null;
  cached: boolean;
}

/** serde 外部标签 enum：恰好一个顶层 key。 */
export type PipelineEventPayload =
  | { BatchStarted: { total: number } }
  | { FileUpdated: FileUpdatePayload }
  | {
      BatchFinished: {
        done: number;
        failed: number;
        cached: number;
        cancelled: boolean;
      };
    }
  | { LogLine: string };

export interface Row {
  path: string;
  file_name: string;
  state: FileState;
  judgment: FileJudgment | null;
  error: string | null;
  elapsed_ms: number | null;
  cached: boolean;
}

export interface RowsState {
  /** 插入顺序的 path 列表（表格行序） */
  order: string[];
  byPath: Readonly<Record<string, Row>>;
  total: number;
  running: boolean;
  /** LogLine 尾巴（提示/诊断），新的在前 */
  logs: string[];
}

export const emptyRows: RowsState = {
  order: [],
  byPath: {},
  total: 0,
  running: false,
  logs: [],
};

const LOG_CAP = 100;

export type RowsAction =
  | { type: "filesAdded"; paths: string[] }
  | { type: "pipelineEvent"; payload: PipelineEventPayload }
  | { type: "reset" };

function rowFromPath(path: string): Row {
  const file_name = path.split(/[\\/]/).pop() ?? path;
  return {
    path,
    file_name,
    state: "pending",
    judgment: null,
    error: null,
    elapsed_ms: null,
    cached: false,
  };
}

export function rowsReducer(state: RowsState, action: RowsAction): RowsState {
  switch (action.type) {
    case "filesAdded": {
      const byPath = { ...state.byPath };
      const order = [...state.order];
      for (const p of action.paths) {
        if (!byPath[p]) {
          byPath[p] = rowFromPath(p);
          order.push(p);
        }
      }
      return { ...state, byPath, order };
    }
    case "pipelineEvent":
      return reduceEvent(state, action.payload);
    case "reset":
      return emptyRows;
  }
}

function reduceEvent(state: RowsState, payload: PipelineEventPayload): RowsState {
  if ("BatchStarted" in payload) {
    return { ...state, running: true, total: payload.BatchStarted.total };
  }
  if ("FileUpdated" in payload) {
    const u = payload.FileUpdated;
    const prev = state.byPath[u.path] ?? rowFromPath(u.path);
    const row: Row = {
      ...prev,
      state: u.state,
      judgment: u.judgment ?? prev.judgment,
      error: u.error,
      elapsed_ms: u.elapsed_ms,
      cached: u.cached,
    };
    return {
      ...state,
      byPath: { ...state.byPath, [u.path]: row },
      order: state.byPath[u.path] ? state.order : [...state.order, u.path],
    };
  }
  if ("BatchFinished" in payload) {
    return { ...state, running: false };
  }
  // LogLine
  return { ...state, logs: [payload.LogLine, ...state.logs].slice(0, LOG_CAP) };
}

// ── 派生统计（单一事实来源：直接从行状态算） ──

export function countByState(state: RowsState, target: FileState): number {
  return state.order.filter((p) => state.byPath[p]?.state === target).length;
}

export function countCached(state: RowsState): number {
  return state.order.filter((p) => state.byPath[p]?.cached).length;
}

export function hasDone(state: RowsState): boolean {
  return countByState(state, "done") > 0;
}

/** 状态中文文案（与旧版 store.rs 一致） */
export const STATUS_TEXT: Record<FileState, string> = {
  pending: "待判定",
  extracting: "提取中",
  queued: "排队中",
  judging: "判定中",
  done: "完成",
  failed: "失败",
};

/** 资历 key → label 由 get_meta 提供；缺省兜底显示 key 本身 */
export function seniorityDisplay(
  key: string | null,
  labels: Readonly<Record<string, string>>,
): string {
  if (!key) return "-";
  return labels[key] ?? key;
}

// ── 搜索 / 筛选 ──

export type StatusFilter = "all" | "running" | "done" | "failed";

export const STATUS_FILTER_TEXT: Record<StatusFilter, string> = {
  all: "全部状态",
  running: "进行中",
  done: "完成",
  failed: "失败",
};

const RUNNING_STATES: ReadonlySet<FileState> = new Set([
  "pending",
  "extracting",
  "queued",
  "judging",
]);

/** 按文件名关键字 + 状态筛选，返回按插入序的行（纯函数）。 */
export function filterRows(
  state: RowsState,
  query: string,
  status: StatusFilter,
): Row[] {
  const q = query.trim().toLowerCase();
  return state.order.map((p) => state.byPath[p]).filter((r) => {
    if (q && !r.file_name.toLowerCase().includes(q)) return false;
    if (status === "done" && r.state !== "done") return false;
    if (status === "failed" && r.state !== "failed") return false;
    if (status === "running" && !RUNNING_STATES.has(r.state)) return false;
    return true;
  });
}
