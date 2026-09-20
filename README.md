# jev-resume

Jev 简历批量分类桌面工具（Windows / macOS / Linux）：导入一个文件夹的简历（PDF / DOCX / TXT，支持拖拽整文件夹进窗口），后台并发送入 TypeSafe Jev 决策模型，输出每个回答的四维判定：

| 维度 | 形式 | 说明 |
| --- | --- | --- |
| 岗位分类 | choice | 后端/前端/移动端/算法/数据/测试/运维/产品/其他（判据可在 `criteria.toml` 中编辑） |
| 资历级别 | choice | 实习 / 初级1-3年 / 中级3-5年 / 高级5-10年 / 专家10年+ |
| 技术强度 | score 0-10 | 项目可量化程度、技术深度、主导性 |
| 注水嫌疑 | noul 0-1 | 时间线矛盾、头衔失实、数字堆砌等信号 |

批次可随时取消；结果本地缓存（按内容哈希），改判据自动失效重判；可导出 CSV（Excel 兼容）/ JSON。

> 判定由 AI 模型生成，仅供参考，不构成招聘决策依据。简历内容会发送至 api.typesafe.ai 进行判定，请遵守当地个人信息保护法规，仅处理你有权处理的简历。

## 架构

- `crates/jev-core` — 无 UI 依赖的核心库：三格式文本提取（pdfium-render / zip+quick-xml / 原文）、Jev 客户端（容错解析 + 429/5xx 退避重试）、批量流水线（Semaphore 并发 + 内容哈希缓存 + 取消）、判据 TOML（版本化）、CSV/JSON 导出。全量单元测试。
- `crates/jev-app` — Tauri 2 桌面应用：Rust 命令层（`src/commands.rs`，批次线程 + 事件泵）+ `ui/` 下的 React + Vite + TypeScript 前端（结果表 / 工具栏 / 状态栏）。

## 本地开发

```bash
# 1. Rust 1.98.1（rust-toolchain.toml 已钉住）与 Node 20+
rustup toolchain install 1.98.1

# 2. pdfium 动态库（脚本会放到 vendor/ 与 target/{debug,release}/ 旁，无需环境变量）
./scripts/fetch-pdfium.sh

# 3. 测试核心与前端
cargo test -p jev-core
npm install
npm test

# 4. 运行桌面应用（自动起 Vite dev server）
npm run dev
```

## API Key

**推荐**：应用内点「设置」（未配置 Key 时顶部提示可直接点击）填入 TypeSafe API Key，保存即生效。也可以直接编辑配置文件：

- macOS: `~/Library/Application Support/jev-resume/config.toml`
- Windows: `%APPDATA%\jev-resume\config.toml`
- Linux: `~/.config/jev-resume/config.toml`

```toml
api_key = "apikey_..."
base_url = "https://api.typesafe.ai"
model = "jev-latest"
concurrency = 4
```

改配置文件后无需重启，下一批次即生效。

## 判据校准

分类体系在 `config.toml` 同目录的 `criteria.toml`：9 个默认分类的 `description` 是判定标准本身，改措辞即改判定；每次保存自动递增版本号，旧缓存自动失效。与 zhihu-jev 相同的校准流程：拿 ~20 份你有把握的简历当测试集，只改判据措辞，直到判定与你的判断一致。

## 安装包（GitHub Release）

推送 `v*` tag（或手动 workflow_dispatch）触发三平台发布构建，产物自动挂到 GitHub Release：

- macOS: `.dmg` / `.app.tar.gz`（未签名，首次打开需右键 → 打开）
- Windows: `-setup.exe` (NSIS) / `.msi`
- Linux: `.deb` / `.AppImage` / `.rpm`

pdfium 动态库已打入安装包（`bundle.resources`），安装后无需任何环境变量。

## License

MIT
