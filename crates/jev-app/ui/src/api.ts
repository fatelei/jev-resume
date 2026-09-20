//! Tauri 后端调用封装（类型安全 invoke + 事件订阅 + 对话框）。

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";

import type { PipelineEventPayload } from "./rows";

export interface Meta {
  seniority_labels: [string, string][];
  config_path: string | null;
  has_api_key: boolean;
}

/** 导出行（对齐 jev-core export::ExportRow 的 serde 字段名） */
export interface ExportRowInput {
  file_name: string;
  status: string;
  role_category: string;
  seniority: string;
  strength: number | null;
  inflation: number | null;
  elapsed_ms: number | null;
  cached: boolean;
  note: string;
}

export const getMeta = () => invoke<Meta>("get_meta");

export const collectFiles = (paths: string[]) =>
  invoke<string[]>("collect_files", { paths });

export const startBatch = (files: string[]) =>
  invoke<void>("start_batch", { files });

export const cancelBatch = () => invoke<void>("cancel_batch");

export const exportResults = (
  rows: ExportRowInput[],
  path: string,
  format: "csv" | "json",
) => invoke<string>("export_results", { rows, path, format });

export const listenPipeline = (
  onEvent: (payload: PipelineEventPayload) => void,
): Promise<UnlistenFn> =>
  listen<PipelineEventPayload>("pipeline-event", (ev) => onEvent(ev.payload));

// ── 对话框（rfd 限制: 文件与文件夹不能混选，文件夹用拖拽兜底） ──

export async function pickResumeFiles(): Promise<string[] | null> {
  const picked = await open({
    multiple: true,
    title: "选择简历文件",
    filters: [{ name: "简历", extensions: ["pdf", "docx", "txt"] }],
  });
  if (!picked) return null;
  return Array.isArray(picked) ? picked : [picked];
}

export async function pickResumeFolder(): Promise<string | null> {
  const picked = await open({
    directory: true,
    multiple: false,
    title: "选择简历文件夹",
  });
  return picked ?? null;
}

export async function pickSavePath(
  defaultName: string,
): Promise<{ path: string; format: "csv" | "json" } | null> {
  const picked = await save({
    defaultPath: defaultName,
    filters: [
      { name: "CSV (Excel 兼容)", extensions: ["csv"] },
      { name: "JSON", extensions: ["json"] },
    ],
  });
  if (!picked) return null;
  const format = picked.toLowerCase().endsWith(".json") ? "json" : "csv";
  return { path: picked, format };
}
