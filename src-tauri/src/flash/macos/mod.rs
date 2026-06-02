//! macOS-specific flash implementation: Security.framework AuthorizationCreate + authopen `-extauth`
//! for privilege escalation when writing to block devices.

mod authorization;
mod bindings;
mod writer;

pub use authorization::request_authorization;
pub use writer::flash_image;
