//! Protected Core：§322 的五个字段槽位与 §321 的五类语义（设计 §3、§4）。
//!
//! 本 crate 是 ENG-001 方向 (a) 的强制面：学习侧（`continuum-learning`）只能读
//! `ProtectedCore` 的指纹，看不到它的私有字段，也没有任何修改既有核心的入口。
//! 本 crate 的 workspace 内依赖为空数组，这是硬断言：任何指向 workspace 内别的
//! crate 的直接边都会让 `continuum-runtime` 的依赖方向用例变红。
//!
//! 五个字段私有，唯一构造点是 `seal(ProtectedCoreParts)`；封存后的核心只提供五个
//! 同名访问器与 `fingerprint()`。核心的规范编码（五组逐组编入组名、状态、版本与
//! 摘要）只在本 crate 的当前版本内解释，**不承诺跨版本稳定**。
