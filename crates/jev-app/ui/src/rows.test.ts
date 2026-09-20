//! rowsReducer 单测：事件 JSON 形状与 jev-core serde 输出逐字对齐
//! （外部标签 enum：{"FileUpdated": {...}}，不是 {type: "..."}）。

import { describe, expect, it } from "vitest";

import {
  countByState,
  countCached,
  emptyRows,
  rowsReducer,
  seniorityDisplay,
  type RowsState,
} from "./rows";

function reduceAll(state: RowsState, payloads: unknown[]): RowsState {
  return payloads.reduce<RowsState>(
    (s, payload) => rowsReducer(s, { type: "pipelineEvent", payload: payload as never }),
    state,
  );
}

describe("rowsReducer", () => {
  it("filesAdded 按 path 去重并以插入序展开", () => {
    const s1 = rowsReducer(emptyRows, { type: "filesAdded", paths: ["/a.pdf", "/b.docx"] });
    const s2 = rowsReducer(s1, { type: "filesAdded", paths: ["/a.pdf", "/c.txt"] });
    expect(s2.order).toEqual(["/a.pdf", "/b.docx", "/c.txt"]);
    expect(s2.byPath["/c.txt"].state).toBe("pending");
  });

  it("消费真实形状的 BatchStarted/FileUpdated/BatchFinished", () => {
    const s = reduceAll(emptyRows, [
      { BatchStarted: { total: 2 } },
      {
        FileUpdated: {
          path: "/tmp/张三_后端.pdf",
          state: "done",
          judgment: {
            role_category: "backend",
            role_confidence: 0.87,
            seniority: "mid_3_5",
            strength: 6.5,
            inflation: 0.1,
            model: "jev-latest",
            input_tokens: 1200,
            output_tokens: 80,
          },
          error: null,
          elapsed_ms: 830,
          cached: false,
        },
      },
      {
        FileUpdated: {
          path: "/tmp/李四.docx",
          state: "failed",
          judgment: null,
          error: "疑似扫描件, 无文本层",
          elapsed_ms: null,
          cached: false,
        },
      },
      { BatchFinished: { done: 1, failed: 1, cached: 0, cancelled: false } },
    ]);

    expect(s.running).toBe(false);
    expect(s.total).toBe(2);
    expect(countByState(s, "done")).toBe(1);
    expect(countByState(s, "failed")).toBe(1);
    expect(countCached(s)).toBe(0);
    expect(s.byPath["/tmp/张三_后端.pdf"].judgment?.role_category).toBe("backend");
    expect(s.byPath["/tmp/李四.docx"].error).toBe("疑似扫描件, 无文本层");
  });

  it("cached 行计入缓存计数", () => {
    const s = reduceAll(emptyRows, [
      { BatchStarted: { total: 1 } },
      {
        FileUpdated: {
          path: "/tmp/缓存命中.pdf",
          state: "done",
          judgment: null,
          error: null,
          elapsed_ms: 12,
          cached: true,
        },
      },
    ]);
    expect(countCached(s)).toBe(1);
  });

  it("FileUpdated 先于 filesAdded 到达时也能落行（健壮性）", () => {
    const s = reduceAll(emptyRows, [
      {
        FileUpdated: {
          path: "/tmp/晚到.pdf",
          state: "queued",
          judgment: null,
          error: null,
          elapsed_ms: null,
          cached: false,
        },
      },
    ]);
    expect(s.order).toEqual(["/tmp/晚到.pdf"]);
  });

  it("LogLine 进日志尾巴且封顶", () => {
    let s = emptyRows;
    for (let i = 0; i < 120; i++) {
      s = rowsReducer(s, { type: "pipelineEvent", payload: { LogLine: `log ${i}` } });
    }
    expect(s.logs.length).toBe(100);
    expect(s.logs[0]).toBe("log 119");
  });

  it("reducer 不可变：旧 state 不被就地修改", () => {
    const s1 = rowsReducer(emptyRows, { type: "filesAdded", paths: ["/a.pdf"] });
    const s2 = rowsReducer(s1, {
      type: "pipelineEvent",
      payload: {
        FileUpdated: {
          path: "/a.pdf",
          state: "done",
          judgment: null,
          error: null,
          elapsed_ms: 5,
          cached: false,
        },
      },
    });
    expect(s1.byPath["/a.pdf"].state).toBe("pending");
    expect(s2.byPath["/a.pdf"].state).toBe("done");
    expect(s1.order).toBe(s2.order); // 未增行时 order 引用不变
  });
});

describe("seniorityDisplay", () => {
  it("key 映射 label，未知 key 兜底原样，空显示 -", () => {
    const labels = { mid_3_5: "中级 (3-5年)" };
    expect(seniorityDisplay("mid_3_5", labels)).toBe("中级 (3-5年)");
    expect(seniorityDisplay("unknown_key", labels)).toBe("unknown_key");
    expect(seniorityDisplay(null, labels)).toBe("-");
  });
});
