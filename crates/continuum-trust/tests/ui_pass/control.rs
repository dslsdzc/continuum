//! §9.2 s：合法调用的正控制，证明 q/r 的红不是样例自身的语法错。

use continuum_trust::{ContextProvenance, PackageShape, label, validate};

fn main() {
    let p = ContextProvenance::web_untrusted();
    let t = label(&p);
    let shape = PackageShape {
        instructions: 0,
        references: 1,
        artifacts: 0,
        memories: 0,
    };
    assert!(validate(shape, &[p], &[t]).is_ok());
}
