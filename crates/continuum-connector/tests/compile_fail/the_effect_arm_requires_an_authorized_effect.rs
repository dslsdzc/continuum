// 应编译失败：效应臂的字段类型是 `AuthorizedEffect`，裸 `Capability` 填不进去。
//
// **这条样例钉住的是哪一件事，要说准**（设计 §5.1）：它钉的**只是类型层的一条**
// ——`ConnectorAuthorization::Effect` 变体的字段类型。它**钉不住**「效应型操作不得
// 用裸能力驱动」：后者是**运行期**核对（入口第 3 步），照片是 `EffectAuthorizationRequired`
// 那一条（`tests/invoke.rs`）。**两件事不得混为一谈**：把裸能力换成 `AuthorizedEffect`
// 能让本样例编译过，但一枚 kind 正确、却是裸能力驱动的效应型操作，仍要由第 3 步在
// 运行期拒——类型的「收什么」与运行期的「走没走强制点 (2)」是两道关。

use continuum_capability::Capability;
use continuum_connector::ConnectorAuthorization;

fn drive_with_a_bare_capability(cap: Capability) -> ConnectorAuthorization {
    ConnectorAuthorization::Effect(cap)
}

fn main() {}
