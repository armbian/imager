//! Security framework FFI bindings
//!
//! C bindings for macOS Security.framework authorization.

use std::ffi::c_void;

// Security framework bindings
#[link(name = "Security", kind = "framework")]
extern "C" {
    pub fn AuthorizationCreate(
        rights: *const AuthorizationRights,
        environment: *const AuthorizationEnvironment,
        flags: u32,
        authorization: *mut AuthorizationRef,
    ) -> i32;

    pub fn AuthorizationFree(authorization: AuthorizationRef, flags: u32) -> i32;

    pub fn AuthorizationMakeExternalForm(
        authorization: AuthorizationRef,
        external_form: *mut AuthorizationExternalForm,
    ) -> i32;
}

/// Authorization reference (opaque pointer)
pub type AuthorizationRef = *mut c_void;

/// Wrapper to make AuthorizationRef Send+Sync safe
pub struct SafeAuthRef(pub AuthorizationRef);
unsafe impl Send for SafeAuthRef {}
unsafe impl Sync for SafeAuthRef {}

/// Authorization item for rights requests
#[repr(C)]
pub struct AuthorizationItem {
    pub name: *const i8,
    pub value_length: usize,
    pub value: *mut c_void,
    pub flags: u32,
}

/// Authorization rights structure
#[repr(C)]
pub struct AuthorizationRights {
    pub count: u32,
    pub items: *mut AuthorizationItem,
}

/// Authorization environment structure
#[repr(C)]
pub struct AuthorizationEnvironment {
    pub count: u32,
    pub items: *mut AuthorizationItem,
}

/// External form for passing authorization between processes
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AuthorizationExternalForm {
    pub bytes: [u8; 32],
}

// Authorization flags
pub const K_AUTHORIZATION_FLAG_INTERACTION_ALLOWED: u32 = 1 << 0;
pub const K_AUTHORIZATION_FLAG_EXTEND_RIGHTS: u32 = 1 << 1;
pub const K_AUTHORIZATION_FLAG_PRE_AUTHORIZE: u32 = 1 << 4;
