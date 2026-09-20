//! 工具栏：导入（文件/文件夹）、取消、导出、无 Key 提示。

interface ToolbarProps {
  running: boolean;
  canExport: boolean;
  hasApiKey: boolean;
  configPath: string | null;
  onPickFiles: () => void;
  onPickFolder: () => void;
  onCancel: () => void;
  onExport: () => void;
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
      <div className="toolbar-hint">
        支持 PDF / DOCX / TXT（单文件或整文件夹），结果本地缓存
      </div>
      {!props.hasApiKey && (
        <div className="key-warning" title={props.configPath ?? ""}>
          未配置 API Key，请编辑 {props.configPath ?? "配置文件"}
        </div>
      )}
    </div>
  );
}
