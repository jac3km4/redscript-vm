use redscript::Ref;
use thiserror::Error;

pub type RuntimeResult<A, E = RuntimeError> = Result<A, E>;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("null pointer dereference")]
    NullPointer,
    #[error("native {0} is not defined")]
    UndefinedNative(Ref<str>),
    #[error("unsupported assingment operand")]
    UnsupportedAssignmentOperand,
    #[error("invalid parameters in interop call")]
    InvalidInteropParameters,
}
