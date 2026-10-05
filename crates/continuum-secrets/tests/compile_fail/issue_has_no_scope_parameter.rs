// 应编译失败：唯一取得凭据的路径只收能力本身，调用方**没有可以指定另一个作用域的
// 位置**（设计 §5.2 第一条：作用域由能力决定，调用方无从放宽）。
//
// 判据是编译失败，且失败必须落在**实参个数**上（E0061）——不是「没有这个方法」之类的
// 别的错：`issue` 存在，只是收不下第三个实参。

use continuum_capability::Capability;
use continuum_secrets::SecretsRuntime;

fn issue_for_another_scope(rt: &SecretsRuntime, cap: &Capability) {
    let _ = rt.issue(cap, "repo/Y", 0);
}

fn main() {}
