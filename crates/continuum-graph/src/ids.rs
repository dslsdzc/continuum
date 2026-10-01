//! 图的标识类型。

use serde::{Deserialize, Serialize};

// 派生 `Ord`：id 的字典序构成全序，排序场景需要它。
// 实现 `Display`：`GraphError` 的格式串以 `{id}` / `{from}` / `{to}` 引用 id，
// thiserror 要求这些字段实现 `Display`。两者对所有 id 类型统一给出。
macro_rules! id_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        // 便于按字面量构造，例如 `graph.add_port("n1", port)`。
        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }
    };
}

id_type!(GraphId, "ADFIR 图标识。");
id_type!(NodeId, "节点标识。");
id_type!(ContractIdRef, "图所属的 Contract 标识（§235 的 contract_id）。");
