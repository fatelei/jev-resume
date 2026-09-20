//! 工具栏：导入（文件/文件夹）、取消、导出、设置、无 Key 提示。

interface ToolbarProps {
  running: boolean;
  canExport: boolean;
  hasApiKey: boolean;
  configPath: string | null;
  onPickFiles: () => void;
  onPickFolder: () => void;
  onCancel: () => void;
  onExport: () => void;
  onOpenSettings: () => void;
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
      <button
        className={`btn ${props.hasApiKey ? "" : "warn"}`}
        onClick={props.onOpenSettings}
        title="配置 API Key"
      >
        设置{props.hasApiKey ? "" : " (未配置 Key)"}
      </button>
      <div className="toolbar-hint">
        支持 PDF / DOCX / TXT（单文件或整文件夹），结果本地缓存
      </div>
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
