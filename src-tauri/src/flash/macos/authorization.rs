//! Authorization management module
//!
//! Handles macOS authorization requests and storage.

use once_cell::sync::Lazy;
use std::sync::Mutex;

use crate::{log_debug, log_error, log_info};

use super::bindings::{
    AuthorizationCreate, AuthorizationExternalForm, AuthorizationFree, AuthorizationItem,
    AuthorizationMakeExternalForm, AuthorizationRef, AuthorizationRights, SafeAuthRef,
    K_AUTHORIZATION_FLAG_EXTEND_RIGHTS, K_AUTHORIZATION_FLAG_INTERACTION_ALLOWED,
    K_AUTHORIZATION_FLAG_PRE_AUTHORIZE,
};

const MODULE: &str = "flash::macos::auth";

/// Saved authorization state between request and flash
pub struct SavedAuthorization {
    pub auth_ref: SafeAuthRef,
    pub external_form: AuthorizationExternalForm,
    pub device_path: String,
}

/// Global state to store authorization between request and flash
pub static SAVED_AUTH: Lazy<Mutex<Option<SavedAuthorization>>> = Lazy::new(|| Mutex::new(None));

/// Request authorization BEFORE download starts
/// This shows the dialog immediately when user clicks Write
pub fn request_authorization(device_path: &str) -> Result<bool, String> {
    // Use raw disk path
    let raw_device = device_path.replace("/dev/disk", "/dev/rdisk");

    unsafe {
        let right_name = format!("sys.openfile.readwrite.{}", raw_device);
        let right_name_cstr = std::ffi::CString::new(right_name.clone())
            .map_err(|_| "Invalid device path: contains null byte".to_string())?;

        let mut item = AuthorizationItem {
            name: right_name_cstr.as_ptr(),
            value_length: 0,
            value: std::ptr::null_mut(),
            flags: 0,
        };

        let rights = AuthorizationRights {
            count: 1,
            items: &mut item,
        };

        let flags = K_AUTHORIZATION_FLAG_INTERACTION_ALLOWED
            | K_AUTHORIZATION_FLAG_EXTEND_RIGHTS
            | K_AUTHORIZATION_FLAG_PRE_AUTHORIZE;

        let mut auth_ref: AuthorizationRef = std::ptr::null_mut();

        log_info!(MODULE, "Requesting authorization for device: {}", raw_device);
        log_debug!(MODULE, "Right name: {}", right_name);

        let status = AuthorizationCreate(
            &rights as *const AuthorizationRights,
            std::ptr::null(),
            flags,
            &mut auth_ref,
        );

        log_debug!(MODULE, "AuthorizationCreate returned status: {}", status);

        if status != 0 {
            // User cancelled or error
            return Ok(false);
        }

        // Convert to external form
        let mut external_form = AuthorizationExternalForm::default();
        let status = AuthorizationMakeExternalForm(auth_ref, &mut external_form);

        if status != 0 {
            AuthorizationFree(auth_ref, 0);
            log_error!(MODULE, "AuthorizationMakeExternalForm failed: {}", status);
            return Err(format!("Failed to create authorization: {}", status));
        }

        // Save authorization for later use - KEEP auth_ref alive!
        let mut saved = SAVED_AUTH.lock().unwrap();
        *saved = Some(SavedAuthorization {
            auth_ref: SafeAuthRef(auth_ref),
            external_form,
            device_path: raw_device,
        });

        log_info!(MODULE, "Authorization saved successfully");
        Ok(true)
    }
}

/// Free an authorization reference
pub unsafe fn free_authorization(auth_ref: AuthorizationRef) {
    log_debug!(MODULE, "Freeing authorization ref");
    AuthorizationFree(auth_ref, 0);
}
