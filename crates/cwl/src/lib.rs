pub use cwl_core::*;

#[cfg(feature = "execution")]
pub mod execution {
    pub use cwl_execution::*;
}

#[cfg(feature = "annotation")]
pub mod annotation {
    pub use cwl_annotation::*;
}
