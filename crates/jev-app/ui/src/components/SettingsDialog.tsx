//! 设置弹窗：页面内配置 API Key / base_url / model / 并发。

import { useEffect, useState } from "react";

import { getConfig, saveConfig, defaultConfig, type AppConfig } from "../api";

interface SettingsDialogProps {
  configPath: string | null;
  onClose: () => void;
  onSaved: (config: AppConfig) => void;
}

export function SettingsDialog(props: SettingsDialogProps) {
  const [draft, setDraft] = useState<AppConfig>(defaultConfig());
  const [showKey, setShowKey] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    getConfig()
      .then((c) => {
        if (!disposed) setDraft(c);
      })
      .catch((e) => setError(String(e)));
    return () => {
      disposed = true;
    };
  }, []);

  const set = <K extends keyof AppConfig>(key: K, value: AppConfig[K]) => {
    setDraft((d) => ({ ...d, [key]: value }));
  };

  const onSave = async () => {
    const concurrency = Math.floor(draft.concurrency) || 1;
    const normalized: AppConfig = {
      ...draft,
      api_key: draft.api_key.trim(),
      base_url: draft.base_url.trim() || defaultConfig().base_url,
      model: draft.model.trim() || defaultConfig().model,
      concurrency: Math.min(Math.max(concurrency, 1), 32),
    };
    setSaving(true);
    setError(null);
    try {
      await saveConfig(normalized);
      props.onSaved(normalized);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="modal-mask" onClick={props.onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2>设置</h2>

        <label className="field">
          <span>TypeSafe API Key</span>
          <div className="key-row">
            <input
              type={showKey ? "text" : "password"}
              value={draft.api_key}
              placeholder="apikey_..."
              onChange={(e) => set("api_key", e.target.value)}
              autoFocus
            />
            <button className="btn tiny" onClick={() => setShowKey((v) => !v)}>
              {showKey ? "隐藏" : "显示"}
            </button>
          </div>
        </label>

        <label className="field">
          <span>Base URL</span>
          <input
            value={draft.base_url}
            onChange={(e) => set("base_url", e.target.value)}
          />
        </label>

        <label className="field">
          <span>模型</span>
          <input
            value={draft.model}
            onChange={(e) => set("model", e.target.value)}
          />
        </label>

        <label className="field">
          <span>并发数 (1-32)</span>
          <input
            type="number"
            min={1}
            max={32}
            value={draft.concurrency}
            onChange={(e) => set("concurrency", Number(e.target.value))}
          />
        </label>

        <p className="field-hint">Key 保存在本机 {props.configPath ?? "配置文件"}（unix 0600），保存后下一批次生效。</p>
        {error && <p className="form-error">{error}</p>}

        <div className="modal-actions">
          <button className="btn" onClick={props.onClose} disabled={saving}>
            取消
          </button>
          <button className="btn primary" onClick={onSave} disabled={saving}>
            {saving ? "保存中…" : "保存"}
          </button>
        </div>
      </div>
    </div>
  );
}
