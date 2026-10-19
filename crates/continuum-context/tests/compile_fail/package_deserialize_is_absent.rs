// 应编译失败（J1-c）：`ContextPackage` 没有 `Deserialize`，反序列化是第二条构造路径。
fn main() {
    let _: continuum_context::ContextPackage =
        serde_json::from_str("{}").expect("空对象不可能装出六字段");
}
