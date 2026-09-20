//! 应用骨架：事件订阅 + 拖拽 + 各操作的处理编排。

import { useCallback, useEffect, useReducer, useMemo, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

import {
  cancelBatch,
  collectFiles,
  exportResults,
  getMeta,
  listenPipeline,
  pickResumeFiles,
  pickResumeFolder,
  pickSavePath,
  startBatch,
  type ExportRowInput,
  type Meta,
} from "./api";
import {
  STATUS_TEXT,
  countByState,
  emptyRows,
  rowsReducer,
  seniorityDisplay,
} from "./rows";
import { Toolbar } from "./components/Toolbar";
import { ResultsTable } from "./components/ResultsTable";
import { StatusBar } from "./components/StatusBar";

export default function App() {
  const [rowsState, dispatch] = useReducer(rowsReducer, emptyRows);
  const [meta, setMeta] = useState<Meta | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const seniorityLabels = useMemo(() => {
    const map: Record<string, string> = {};
    for (const [key, label] of meta?.seniority_labels ?? []) {
      map[key] = label;
    }
    return map;
  }, [meta]);

  // 启动：读 meta + 订阅流水线事件 + 拖拽
  // eslint-disable-next-line react-hooks/exhaustive-deps -- importPaths 为稳定 useCallback([])
  useEffect(() => {
    let disposed = false;
    const unlistens: Array<() => void> = [];

    getMeta()
      .then((m) => {
        if (!disposed) setMeta(m);
      })
      .catch((e) => setNotice(`读取配置失败: ${String(e)}`));

    listenPipeline((payload) => dispatch({ type: "pipelineEvent", payload }))
      .then((un) => {
        if (disposed) un();
        else unlistens.push(un);
      })
      .catch((e) => setNotice(`事件订阅失败: ${String(e)}`));

    getCurrentWebview()
      .onDragDropEvent((ev) => {
        if (ev.payload.type === "drop" && ev.payload.paths.length > 0) {
          void importPaths(ev.payload.paths);
        }
      })
      .then((un) => {
        if (disposed) un();
        else unlistens.push(un);
      })
      .catch((e) => setNotice(`拖拽初始化失败: ${String(e)}`));

    return () => {
      disposed = true;
      for (const un of unlistens) un();
    };
  }, []);

  /** 路径（文件或文件夹）→ 展开 → 入表 → 起批次 */
  const importPaths = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0) return;
      try {
        setNotice(null);
        const files = await collectFiles(paths);
        if (files.length === 0) {
          setNotice("所选路径里没有 PDF / DOCX / TXT 简历");
          return;
        }
        dispatch({ type: "filesAdded", paths: files });
        await startBatch(files);
      } catch (e) {
        setNotice(String(e));
      }
    },
    [],
  );

  const onPickFiles = useCallback(async () => {
    const picked = await pickResumeFiles().catch((e) => {
      setNotice(String(e));
      return null;
    });
    if (picked) void importPaths(picked);
  }, [importPaths]);

  const onPickFolder = useCallback(async () => {
    const folder = await pickResumeFolder().catch((e) => {
      setNotice(String(e));
      return null;
    });
    if (folder) void importPaths([folder]);
  }, [importPaths]);

  const onCancel = useCallback(() => {
    cancelBatch().catch((e) => setNotice(`取消失败: ${String(e)}`));
  }, []);

  const onExport = useCallback(async () => {
    try {
      const picked = await pickSavePath("resume-results.csv");
      if (!picked) return;
      const exportRows: ExportRowInput[] = rowsState.order.map((p) => {
        const row = rowsState.byPath[p];
        return {
          file_name: row.file_name,
          status: STATUS_TEXT[row.state],
          role_category: row.judgment?.role_category ?? "",
          seniority: seniorityDisplay(row.judgment?.seniority ?? null, seniorityLabels),
          strength: row.judgment?.strength ?? null,
          inflation: row.judgment?.inflation ?? null,
          elapsed_ms: row.elapsed_ms,
          cached: row.cached,
          note: row.error ?? "",
        };
      });
      await exportResults(exportRows, picked.path, picked.format);
      setNotice(`已导出 ${exportRows.length} 行 → ${picked.path}`);
    } catch (e) {
      setNotice(`导出失败: ${String(e)}`);
    }
  }, [rowsState, seniorityLabels]);

  const done = countByState(rowsState, "done");

  return (
    <div className="app">
      <Toolbar
        running={rowsState.running}
        canExport={done > 0}
        hasApiKey={meta?.has_api_key ?? true}
        configPath={meta?.config_path ?? null}
        onPickFiles={onPickFiles}
        onPickFolder={onPickFolder}
        onCancel={onCancel}
        onExport={onExport}
      />
      {rowsState.order.length === 0 ? (
        <div className="empty-hint">
          点击「导入文件 / 导入文件夹」，或直接把简历拖进窗口
          （支持 PDF / DOCX / TXT，结果本地缓存）
        </div>
      ) : (
        <ResultsTable state={rowsState} seniorityLabels={seniorityLabels} />
      )}
      <StatusBar state={rowsState} notice={notice} />
    </div>
  );
}
