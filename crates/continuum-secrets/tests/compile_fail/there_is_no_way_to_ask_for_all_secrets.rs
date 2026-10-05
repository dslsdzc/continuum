// 应编译失败：运行时只按能力逐枚签发，没有「取全部」的入口（§51：Agent 不直接获得
// 所有密钥；设计 §5.2 第三条）。
//
// 判据是编译失败，且失败必须落在「没有这个方法」上（E0599），而不是别的错。

use continuum_secrets::SecretsRuntime;

fn ask_for_everything(rt: &SecretsRuntime) {
    let _ = rt.issue_all(0);
}

fn main() {}
