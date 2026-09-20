# jev-resume

Jev 简历批量分类桌面工具（Windows / macOS / Linux）：拖入一个文件夹的简历（PDF / DOCX / TXT），后台并发送入 TypeSafe Jev 决策模型，输出每个回答的四维判定：

| 维度 | 形式 | 说明 |
| --- | --- | --- |
| 岗位分类 | choice | 后端/前端/移动端/算法/数据/测试/运维/产品/其他（可在应用内编辑分类体系） |
| 资历级别 | choice | 实习 / 初级1-3年 / 中级3-5年 / 高级5-10年 / 专家10年+ |
| 技术强度 | score 0-10 | 项目可量化程度、技术深度、主导性 |
| 注水嫌疑 | noul 0-1 | 时间线矛盾、头衔失实、数字堆砌等信号 |

结果本地缓存（按内容哈希），改判据自动失效重判；可导出 CSV（Excel 兼容）/ JSON。

> 判定由 AI 模型生成，仅供参考，不构成招聘决策依据。简历内容会发送至 api.typesafe.ai 进行判定，请遵守当地个人信息保护法规，仅处理你有权处理的简历。

## 架构

- `crates/jev-core` — 无 UI 依赖的核心库：三格式文本提取（pdfium-render / zip+quick-xml / 原文）、Jev 客户端（容错解析 + 429/5xx 退避重试）、批量流水线（Semaphore 并发 + 内容哈希缓存 + 取消）、判据 TOML（版本化）、CSV/JSON 导出。全量单元测试。
- `crates/jev-app` — GPUI 桌面应用（gpui-component 0.6.4 组件库）。

## 本地开发

```bash
# 1. Rust 1.98.1（rust-toolchain.toml 已钉住）
rustup toolchain install 1.98.1

# 2. pdfium 动态库（macOS/Linux/Windows 各有脚本支持）
./scripts/fetch-pdfium.sh
export PDFIUM_DYNAMIC_LIB_PATH="$PWD/vendor/pdfium/lib/libpdfium.dylib"

# 3. 测试核心（不依赖 pdfium 的用例会自动跳过）
cargo test -p jev-core

# 4. 运行桌面应用
cargo run -p jev-app
```

### API Key

首次运行前编辑配置文件（自动生成）填入 TypeSafe API Key：

- macOS: `~/Library/Application Support/jev-resume/config.toml`
- Windows: `%APPDATA%\jev-resume\config.toml`
- Linux: `~/.config/jev-resume/config.toml`

```toml
api_key = "apikey_..."
base_url = "https://api.typesafe.ai"
model = "jev-latest"
concurrency = 4
```

## 判据校准

分类体系在 `config.toml` 同目录的 `criteria.toml`：9 个默认分类的 `description` 是判定标准本身，改措辞即改判定；每次保存自动递增版本号，旧缓存自动失效。与 zhihu-jev 相同的校准流程：拿 ~20 份你有把握的简历当测试集，只改判据措辞，直到判定与你的判断一致。

## License

MIT
