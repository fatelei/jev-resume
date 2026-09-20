//! jev-core: 简历分类核心库（提取 / Jev 判定 / 流水线 / 缓存 / 导出）。
//!
//! 本 crate 零 UI 依赖，全部类型可独立单元测试。
//! 纪律：喂给模型的 state 文本与调试输出所见即所得。

pub mod cache;
pub mod config;
pub mod criteria;
pub mod error;
pub mod export;
pub mod extract;
pub mod hash;
pub mod jev;
pub mod pipeline;
pub mod questions;
pub mod state;

pub use error::{CoreError, CoreResult};
