//! Port 定义与连接兼容性判定。ADFIR 是运行期构造的动态图，
//! 类型校验在连接构造时进行（§239）。

pub mod port;

pub use port::{compatible, Direction, Port, PortError, PortId};
