//! Device writer module
//!
//! Handles opening devices with authorization and writing data.

use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::config;
use crate::flash::{sync_device, unmount_device, FlashState};
use crate::{log_debug, log_error, log_info};

use super::authorization::{free_authorization, SAVED_AUTH};
use super::bindings::AuthorizationRef;

const MODULE: &str = "flash::macos::writer";

/// Wrapper to make auth_ref Send safe
pub struct SendableAuthRef(pub AuthorizationRef);
unsafe impl Send for SendableAuthRef {}

/// Result of opening device - includes auth_ref that must be kept alive
pub struct OpenDeviceResult {
    pub file: File,
    pub auth_ref: SendableAuthRef,
}

/// Open device using previously saved authorization
pub fn open_device_with_saved_auth(device_path: &str) -> Result<OpenDeviceResult, String> {
    let mut saved_guard = SAVED_AUTH.lock().unwrap();
    let auth = saved_guard
        .take()
        .ok_or("No authorization saved - call request_authorization first")?;

    if auth.device_path != device_path {
        // Put it back if mismatch
        *saved_guard = Some(auth);
        return Err(format!(
            "Authorization mismatch: saved for {} but trying to open {}",
            saved_guard.as_ref().unwrap().device_path,
            device_path
        ));
    }

    let external_form = auth.external_form;
    let auth_ref = auth.auth_ref.0;
    drop(saved_guard); // Release lock before fork

    let result = unsafe {
        // Create socket pair for receiving fd
        let mut sock_pair: [i32; 2] = [0; 2];
        if libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, sock_pair.as_mut_ptr()) != 0 {
            return Err("Failed to create socket pair".to_string());
        }

        // Create pipe for stdin (to pass auth)
        let mut stdin_pipe: [i32; 2] = [0; 2];
        if libc::pipe(stdin_pipe.as_mut_ptr()) != 0 {
            libc::close(sock_pair[0]);
            libc::close(sock_pair[1]);
            return Err("Failed to create stdin pipe".to_string());
        }

        let pid = libc::fork();
        if pid < 0 {
            libc::close(sock_pair[0]);
            libc::close(sock_pair[1]);
            libc::close(stdin_pipe[0]);
            libc::close(stdin_pipe[1]);
            return Err("Failed to fork".to_string());
        }

        if pid == 0 {
            // Child process
            libc::close(sock_pair[0]);
            libc::close(stdin_pipe[1]);

            // Redirect stdout to socket
            libc::dup2(sock_pair[1], libc::STDOUT_FILENO);
            // Redirect stdin from pipe
            libc::dup2(stdin_pipe[0], libc::STDIN_FILENO);

            let authopen = std::ffi::CString::new("/usr/libexec/authopen").expect("static string");
            let arg_stdoutpipe = std::ffi::CString::new("-stdoutpipe").expect("static string");
            let arg_extauth = std::ffi::CString::new("-extauth").expect("static string");
            let arg_o = std::ffi::CString::new("-o").expect("static string");
            let arg_mode = std::ffi::CString::new("2").expect("static string");
            let path = match std::ffi::CString::new(device_path) {
                Ok(p) => p,
                Err(_) => libc::_exit(2),
            };

            libc::execl(
                authopen.as_ptr(),
                authopen.as_ptr(),
                arg_stdoutpipe.as_ptr(),
                arg_extauth.as_ptr(),
                arg_o.as_ptr(),
                arg_mode.as_ptr(),
                path.as_ptr(),
                std::ptr::null::<libc::c_char>(),
            );

            libc::_exit(1);
        }

        // Parent process
        libc::close(sock_pair[1]);
        libc::close(stdin_pipe[0]);

        // Write the external auth form to authopen's stdin
        log_debug!(MODULE, "Sending saved authorization to authopen");
        libc::write(
            stdin_pipe[1],
            external_form.bytes.as_ptr() as *const libc::c_void,
            external_form.bytes.len(),
        );
        libc::close(stdin_pipe[1]);

        // Receive file descriptor from authopen
        let buf_size = 64;
        let mut buf = vec![0u8; buf_size];

        let cmsg_size = libc::CMSG_SPACE(std::mem::size_of::<i32>() as u32) as usize;
        let mut cmsg_buf = vec![0u8; cmsg_size];

        let mut iov = libc::iovec {
            iov_base: buf.as_mut_ptr() as *mut libc::c_void,
            iov_len: buf_size,
        };

        let mut msg: libc::msghdr = std::mem::zeroed();
        msg.msg_iov = &mut iov;
        msg.msg_iovlen = 1;
        msg.msg_control = cmsg_buf.as_mut_ptr() as *mut libc::c_void;
        msg.msg_controllen = cmsg_size as u32;

        let size = libc::recvmsg(sock_pair[0], &mut msg, 0);
        log_debug!(MODULE, "recvmsg returned size: {}", size);

        // Wait for child
        let mut status: i32 = 0;
        loop {
            let wpid = libc::waitpid(pid, &mut status, 0);
            if wpid != -1 || *libc::__error() != libc::EINTR {
                break;
            }
        }

        log_debug!(MODULE, "authopen exit code: {}", libc::WEXITSTATUS(status));

        libc::close(sock_pair[0]);

        if size <= 0 {
            return Err(format!(
                "Failed to receive file descriptor (size={})",
                size
            ));
        }

        if libc::WIFEXITED(status) && libc::WEXITSTATUS(status) != 0 {
            return Err(format!(
                "authopen failed with exit code {}",
                libc::WEXITSTATUS(status)
            ));
        }

        // Extract fd from control message
        let cmsg = libc::CMSG_FIRSTHDR(&msg);
        if cmsg.is_null() {
            return Err("No control message received".to_string());
        }

        let cmsg_ref = &*cmsg;
        if cmsg_ref.cmsg_type != libc::SCM_RIGHTS {
            return Err("Unexpected control message type".to_string());
        }

        let fd_ptr = libc::CMSG_DATA(cmsg) as *const i32;
        let fd = *fd_ptr;

        if fd < 0 {
            return Err("Received invalid file descriptor".to_string());
        }

        log_debug!(MODULE, "Successfully received fd: {}", fd);
        OpenDeviceResult {
            file: File::from_raw_fd(fd),
            auth_ref: SendableAuthRef(auth_ref),
        }
    };

    Ok(result)
}

/// Quick erase - write zeros to first portion of device
pub fn quick_erase(device: &mut File, device_fd: i32) -> Result<(), String> {
    let erase_size = config::flash::QUICK_ERASE_SIZE;
    let chunk_size = config::flash::ERASE_CHUNK_SIZE;

    log_info!(
        MODULE,
        "Quick erase: writing zeros to first {} MB",
        erase_size / (1024 * 1024)
    );

    // Seek to beginning
    unsafe {
        libc::lseek(device_fd, 0, libc::SEEK_SET);
    }

    let zero_buffer = vec![0u8; chunk_size];
    let mut erased: usize = 0;

    while erased < erase_size {
        let to_write = std::cmp::min(chunk_size, erase_size - erased);
        device
            .write_all(&zero_buffer[..to_write])
            .map_err(|e| format!("Quick erase failed at byte {}: {}", erased, e))?;
        erased += to_write;
    }

    // Sync the erase
    device.flush().ok();
    unsafe {
        libc::fsync(device_fd);
    }

    // Seek back to beginning for image write
    unsafe {
        libc::lseek(device_fd, 0, libc::SEEK_SET);
    }

    log_info!(MODULE, "Quick erase complete");
    Ok(())
}

/// Flash an image to a block device on macOS
pub async fn flash_image(
    image_path: &PathBuf,
    device_path: &str,
    state: Arc<FlashState>,
    verify: bool,
) -> Result<(), String> {
    state.reset();

    // Get image size
    let image_size = std::fs::metadata(image_path)
        .map_err(|e| format!("Failed to get image size: {}", e))?
        .len();

    state.total_bytes.store(image_size, Ordering::SeqCst);

    // Use raw disk access for better performance
    let raw_device = device_path.replace("/dev/disk", "/dev/rdisk");

    // Unmount the device first
    unmount_device(device_path)?;

    // Small delay to ensure unmount completes
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Open device using saved authorization (no dialog here!)
    log_info!(MODULE, "Opening device with saved authorization");
    let open_result = open_device_with_saved_auth(&raw_device)?;
    let mut device = open_result.file;
    let device_fd = device.as_raw_fd();
    let auth_ref_wrapper = open_result.auth_ref;

    log_debug!(MODULE, "Keeping authorization ref alive during flash");

    // Clear saved auth state (we have the auth_ref now)
    {
        let mut saved = SAVED_AUTH.lock().unwrap();
        *saved = None;
    }

    // Use inner function to do the actual work, then always free auth at the end
    let result =
        do_flash_work(image_path, device_path, &mut device, device_fd, image_size, state, verify)
            .await;

    drop(device);

    // ALWAYS free the authorization - after everything is done
    unsafe {
        free_authorization(auth_ref_wrapper.0);
    }

    result
}

/// Inner function to do flash work
async fn do_flash_work(
    image_path: &PathBuf,
    device_path: &str,
    device: &mut File,
    device_fd: i32,
    image_size: u64,
    state: Arc<FlashState>,
    verify: bool,
) -> Result<(), String> {
    // Quick erase first - clear partition tables and boot sectors
    quick_erase(device, device_fd)?;

    // Open image file
    let mut image_file =
        File::open(image_path).map_err(|e| format!("Failed to open image: {}", e))?;

    // Write image in chunks with progress
    let chunk_size = config::flash::CHUNK_SIZE;
    let mut buffer = vec![0u8; chunk_size];
    let mut written: u64 = 0;

    log_info!(
        MODULE,
        "Starting to write {} bytes ({:.2} GB)",
        image_size,
        image_size as f64 / 1024.0 / 1024.0 / 1024.0
    );

    loop {
        if state.is_cancelled.load(Ordering::SeqCst) {
            return Err("Flash cancelled".to_string());
        }

        let bytes_read = image_file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read image: {}", e))?;

        if bytes_read == 0 {
            break;
        }

        if let Err(e) = device.write_all(&buffer[..bytes_read]) {
            log_error!(
                MODULE,
                "Write error at byte {}/{}: {}",
                written,
                image_size,
                e
            );
            return Err(format!(
                "Failed to write to device at byte {}: {}",
                written, e
            ));
        }

        written += bytes_read as u64;
        state.written_bytes.store(written, Ordering::SeqCst);

        // Log progress every 512MB
        if written % (512 * 1024 * 1024) == 0 {
            log_info!(
                MODULE,
                "Progress: {:.1}%",
                (written as f64 / image_size as f64) * 100.0
            );
        }
    }

    log_info!(MODULE, "Write complete, syncing...");

    // Sync to ensure all data is written
    device.flush().ok();
    unsafe {
        libc::fsync(device_fd);
    }
    sync_device(device_path);

    // Verify if requested - reuse same fd (no additional auth needed)
    if verify {
        log_info!(MODULE, "Starting verification");
        verify_written_data(image_path, device, device_fd, state.clone())?;
    }

    log_info!(MODULE, "Flash complete!");
    Ok(())
}

/// Verify written data by reading back and comparing
fn verify_written_data(
    image_path: &PathBuf,
    device: &mut File,
    device_fd: i32,
    state: Arc<FlashState>,
) -> Result<(), String> {
    // Seek device back to beginning before verification
    unsafe {
        libc::lseek(device_fd, 0, libc::SEEK_SET);
    }

    // Use shared verification logic
    crate::flash::verify::verify_data(image_path, device, state)
}
