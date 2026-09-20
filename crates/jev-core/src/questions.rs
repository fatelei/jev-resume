//! 问题集构建：4 个问题（岗位分类/资历/强度/注水）。
//! 校准 = 改本文件措辞；改 QUESTIONS_SCHEMA_VERSION 即缓存失效。

use std::collections::BTreeMap;

use serde_json::Value;

use crate::criteria::Criteria;
use crate::jev::types::QuestionSpec;

/// 问题措辞/结构版本：任何影响判定的措辞修改都要递增。
pub const QUESTIONS_SCHEMA_VERSION: u64 = 1;

pub const SENIORITY_LABELS: [(&str, &str); 5] = [
    ("intern", "实习生/应届在读"),
    ("junior_1_3", "初级 (1-3年)"),
    ("mid_3_5", "中级 (3-5年)"),
    ("senior_5_10", "高级 (5-10年)"),
    ("expert_10p", "资深/专家 (10年以上)"),
];

pub fn seniority_label(key: &str) -> String {
    SENIORITY_LABELS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, label)| label.to_string())
        .unwrap_or_else(|| key.to_string())
}

pub fn build_questions(criteria: &Criteria) -> BTreeMap<String, QuestionSpec> {
    let mut questions: BTreeMap<String, QuestionSpec> = BTreeMap::new();

    let category_criteria: BTreeMap<String, String> = criteria
        .categories
        .iter()
        .map(|c| (c.key.clone(), c.description.clone()))
        .collect();

    questions.insert(
        "role_category".into(),
        QuestionSpec::Choice {
            instructions: "判断这份简历最匹配的目标岗位类别, 必须从给定选项中选择一个。"
                .to_string()
                + "判据优先级: 1) 最近一段全职工作的技术栈与职责; "
                + "2) 占篇幅最大、描述最具体的项目经历; 3) 技能清单与自我描述。"
                + "按实际经历而非期望职位判断: 不因简历罗列了某项技术就归类, 以项目与工作内容中可验证的使用为准; "
                + "多方向混合时按承载最强、最具体经历的方向判定; "
                + "应届与在读按实习及课程项目的主体方向; 确实无法归类时选 other, 不强行归类。",
            criteria: category_criteria,
        },
    );

    let seniority_criteria: BTreeMap<String, String> = SENIORITY_LABELS
        .iter()
        .map(|(k, label)| match *k {
            "intern" => (
                k.to_string(),
                format!("{label}: 在读或毕业一年内, 以实习与课程项目为主, 无(或极短)全职经历"),
            ),
            "junior_1_3" => (
                k.to_string(),
                format!("{label}: 1到3年全职经验, 在指导下完成明确范围任务, 能独立负责模块"),
            ),
            "mid_3_5" => (
                k.to_string(),
                format!("{label}: 3到5年全职经验, 能独立负责完整功能或子系统, 熟悉领域最佳实践"),
            ),
            "senior_5_10" => (
                k.to_string(),
                format!("{label}: 5到10年全职经验, 主导过核心系统或跨团队项目, 有技术选型与方案设计决策权"),
            ),
            _ => (
                k.to_string(),
                format!("{label}: 有架构级决策、团队管理或行业影响力"),
            ),
        })
        .collect();

    questions.insert(
        "seniority".into(),
        QuestionSpec::Choice {
            instructions: "评估候选人职业资历级别, 必须五选一。依据(按优先级): 实际全职工作年限(多段可累加, 长期空窗不计)、职级轨迹(带团队/主导架构/独立负责核心系统加分)、项目复杂度。不因学历直接加分, 不因头衔含'高级/资深/专家'字样直接定级, 以实际年限与职责为准; 在读或仅有实习经历判 intern; 年限跨档时按主要工作时间落档。".to_string(),
            criteria: seniority_criteria,
        },
    );

    questions.insert(
        "strength".into(),
        QuestionSpec::Score {
            instructions: "对简历的技术竞争力与履历质量打 0-10 分, 只评估可见正文。加分: 项目规模与复杂度可量化(QPS/数据量/用户量)、有含金量的成果(开源、专利、论文、知名系统)、职责与个人贡献清晰、技术深度(原理级描述、权衡与踩坑)。减分: 职责空泛、堆砌技术名词而无实际应用、经历与产出对不上。不因学校或公司名气直接给高分低分, 但名气带来的项目机会本身可以体现。".to_string(),
            criteria: vec![
                "0-2 履历单薄: 几乎无可评估的项目, 职责空泛, 或与目标岗位基本无关".into(),
                "3-4 一般: 有相关经历但描述笼统, 少有量化成果, 主要是跟随性任务".into(),
                "5-6 中等: 项目经历具体, 能看出独立负责的模块, 有部分量化指标或明确产出".into(),
                "7-8 优秀: 多段扎实经历, 项目规模与贡献可量化, 有主导性角色与技术深度".into(),
                "9-10 顶尖: 有可验证的高含金量成果(知名系统/开源影响力/专利论文), 经历完整且贡献突出".into(),
            ],
        },
    );

    questions.insert(
        "inflation".into(),
        QuestionSpec::Noul {
            instructions: "评估这份简历的夸大/注水嫌疑, 输出 0 到 1 之间的数值, 0 为完全可信, 1 为严重注水。注水信号: 时间线重叠或矛盾、头衔与职责明显不符(小团队'总监')、量化数据异常密集或整数化堆砌、技能罗列与项目经历脱节、照抄岗位 JD 话术、夸大表述('业界领先''从零到一主导')却无细节支撑。校正原则: 措辞正式、使用模板、写了量化数字本身不是注水, 要求数字与上下文细节自洽; 对缺失信息(如未写离职原因)不下注水结论, 以正文可见证据为准。".to_string(),
        },
    );

    questions
}

/// 序列化为线上 payload（questions 字段）。
pub fn build_questions_payload(criteria: &Criteria) -> serde_json::Map<String, Value> {
    let mut map = serde_json::Map::new();
    for (name, spec) in build_questions(criteria) {
        map.insert(name, spec.to_json());
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_questions_with_contract_shapes() {
        let c = Criteria::default_toml();
        let q = build_questions_payload(&c);
        assert_eq!(q.len(), 4);
        assert!(q.contains_key("role_category"));
        assert!(q.contains_key("seniority"));
        assert!(q.contains_key("strength"));
        assert!(q.contains_key("inflation"));

        let rc = &q["role_category"];
        assert_eq!(rc["type"], "choice");
        assert!(rc["criteria"].is_object());
        assert_eq!(rc["criteria"].as_object().unwrap().len(), 9);

        let st = &q["strength"];
        assert_eq!(st["type"], "score");
        assert!(st["criteria"].is_array());
        assert_eq!(st["criteria"].as_array().unwrap().len(), 5);

        let inf = &q["inflation"];
        assert_eq!(inf["type"], "noul");
        assert!(inf.get("criteria").is_none());
    }

    #[test]
    fn user_edited_categories_flow_into_criteria() {
        let mut c = Criteria::default_toml();
        c.categories[0].description = "自定义描述: 侧重 Rust 生态".into();
        let q = build_questions_payload(&c);
        assert!(q["role_category"]["criteria"]
            .to_string()
            .contains("侧重 Rust 生态"));
    }
}
