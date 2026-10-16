// 应编译失败：`Evidence` 的八个字段**全部私有**，故**全字段结构体字面量**这条路在
// crate 外走不通（预期是「字段私有」那一类，常见码 E0451；**判据是实跑产出的那份
// `.stderr` 本身，不是这个码**）。
//
// 本份是 P5a 设计「唯一产生点」那条**结构事实**在本块的**使用侧**照片：`Evidence` 的
// 构造只有 `Evidence::from_tool_result` 一条，绕过它自己拼字面量在 `continuum-media`
// 里写不出来。（按内容指：`docs/superpowers/specs/2026-10-08-p5a-verification-and-evidence-design.md`
// §4.4 里「「唯一」在这里是结构事实」那一段；**按内容指、不写行号**——该文件其后被改过，
// 行号漂过两次：本块的设计原引 `:319`，实测那时在 `:324`，本轮再量又在另一行。）
//
// # 本份**只**钉这一条通道（`default` 那条在兄弟样例里）
//
// 计划原写「在同一条样例里同时写字面量与 `Evidence::default()`」——**实跑推翻了它**：
// 两条通道**互相遮蔽**——`Evidence::default()` 那处的错误（E0599，解析一类）一出现，
// 字面量这处的私有性检查（E0451）**根本不报**（三种形态的对照日志见 task 报告）。
// 故本份只写字面量一处，`default` 那条由兄弟样例 `evidence_has_no_default` 单独钉——
// 与 P5a 自己的处置一致（它把这两条拆成两份）。**合并写一份会漏掉另一条的失效**——
// 在这里它是字面意思。
//
// # 它证不到什么（免得被读成覆盖更广）
//
// **它不声称「拿不到一枚 `Evidence`」**：`Evidence::from_tool_result` 是 `pub fn`，
// 构造由**宿主**直接调它完成（设计 §9.2）——**调用点不在本 crate 里**；本份钉的只是
// 「在本 crate 里造不出 `Evidence`」。
//
// # 字段值
//
// 八个字段的值都是普通表达式：`strength` / `scope` 两格用 `Default::default()`——
// 它被 `from_value` 的参数类型定死为 `serde_json::Value`，**不需要在本 crate 里命名
// serde_json**。**不用 `todo!()` 之类的发散（diverge）表达式**：那会让「后面的字段
// 不可达」与「导入未使用」的 lint 混进 `.stderr`（实测过一版，见 task 报告）。
use continuum_verify::{
    Claim, Evidence, EvidenceId, EvidenceProducer, EvidenceScope, EvidenceStrength,
    EvidenceSubject, EvidenceType,
};

fn build_by_literal() -> Evidence {
    Evidence {
        id: EvidenceId::from_str("e-1"),
        evidence_type: EvidenceType::Test,
        subject: EvidenceSubject::Unattached,
        claim: Claim::from_str("测试通过"),
        producer: EvidenceProducer::Human,
        artifact_refs: Vec::new(),
        strength: EvidenceStrength::from_value(Default::default()),
        scope: EvidenceScope::from_value(Default::default()),
    }
}

fn main() {
    // 取一次函数项，免得 `dead_code` 警告混进本样例的 `.stderr`；
    // 函数体照常被类型检查，故上面那次字面量仍是本份要的解析。
    let _ = build_by_literal;
}
