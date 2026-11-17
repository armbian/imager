// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! How a failed or cancelled QDL write is reported, and whether the device may still take commands.

use std::error::Error;
use std::io;

use super::{
    QDLRS_PROGRAM_NAK, QDLRS_RECEIVE_DATA, QDLRS_SEND_DATA, QDLRS_SEND_ZLP, QDLRS_WRITE_OP_NAK,
    QDL_CANCELLED_ERROR, TAG_QDL_DISCONNECTED,
};
use crate::log_warn;

/// A failed `firehose_program_storage` call, classified once at the call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteFailure {
    pub message: String,
    /// The device answered with XML, so the reset-on-drop to EDL may still be sent.
    pub keep_armed: bool,
    /// The `<program>` command itself was refused: the UFS module has no LUN 0 yet.
    pub needs_provisioning: bool,
}

impl WriteFailure {
    pub fn classify(err: &(dyn Error + 'static)) -> Self {
        let chain: Vec<&(dyn Error + 'static)> =
            std::iter::successors(Some(err), |&e| e.source()).collect();
        let messages: Vec<String> = chain.iter().map(|e| e.to_string()).collect();
        let joined = messages.join(": ");

        let aborted = chain.iter().any(|e| {
            e.downcast_ref::<io::Error>()
                .is_some_and(|io| io.kind() == io::ErrorKind::ConnectionAborted)
        });
        let message = if aborted || is_send_failure(&joined) {
            format!("{TAG_QDL_DISCONNECTED} {joined}")
        } else {
            joined
        };

        // Only a bare qdlrs bail! counts: a malformed-ACK error embeds device-controlled text.
        let root = match messages.as_slice() {
            [only] => Some(only.as_str()),
            _ => None,
        };
        let needs_provisioning = root.is_some_and(|m| m.starts_with(QDLRS_PROGRAM_NAK));
        let keep_armed =
            needs_provisioning || root.is_some_and(|m| m.starts_with(QDLRS_WRITE_OP_NAK));

        Self {
            message,
            keep_armed,
            needs_provisioning,
        }
    }

    /// An app-side failure raised before qdlrs touched the device, which can still take commands.
    pub fn before_write(message: String) -> Self {
        Self {
            message,
            keep_armed: true,
            needs_provisioning: false,
        }
    }
}

/// App-side errors (path checks, open, seek, cancel between partitions) keep the device armed.
impl From<String> for WriteFailure {
    fn from(message: String) -> Self {
        Self::before_write(message)
    }
}

fn is_send_failure(msg: &str) -> bool {
    [QDLRS_SEND_DATA, QDLRS_SEND_ZLP]
        .iter()
        .any(|fragment| msg.contains(fragment))
}

// qdlrs emits the receive fragment only as panic text, so it never tags a returned error.
pub fn is_disconnect_panic(msg: &str) -> bool {
    is_send_failure(msg) || msg.contains(QDLRS_RECEIVE_DATA)
}

/// Final result of a QDL flash: only the cancel flag turns an error into a cancel.
pub fn settle(result: Result<(), String>, cancelled: bool) -> Result<(), String> {
    match result {
        Err(e) if cancelled => {
            log_warn!("qdl::outcome", "QDL flash stopped after cancel: {}", e);
            Err(QDL_CANCELLED_ERROR.to_string())
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    #[derive(Debug)]
    struct Ctx {
        msg: String,
        source: Option<Box<dyn Error + Send + Sync>>,
    }

    impl fmt::Display for Ctx {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&self.msg)
        }
    }

    impl Error for Ctx {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            self.source.as_deref().map(|e| e as &(dyn Error + 'static))
        }
    }

    fn msg(text: &str) -> Ctx {
        Ctx {
            msg: text.to_string(),
            source: None,
        }
    }

    fn wrap(text: &str, inner: impl Error + Send + Sync + 'static) -> Ctx {
        Ctx {
            msg: text.to_string(),
            source: Some(Box::new(inner)),
        }
    }

    #[test]
    fn cancel_wins_over_any_failure_and_ok_stays_ok() {
        assert_eq!(settle(Ok(()), true), Ok(()));
        assert_eq!(settle(Ok(()), false), Ok(()));
        for err in ["[QDL_DISCONNECTED] Error sending ZLP", "timeout", ""] {
            assert_eq!(
                settle(Err(err.to_string()), true),
                Err(QDL_CANCELLED_ERROR.to_string())
            );
        }
    }

    #[test]
    fn failures_pass_through_without_a_cancel() {
        for err in ["Failed to write UFS image: x", "Interrupted", "cancelled"] {
            assert_eq!(settle(Err(err.to_string()), false), Err(err.to_string()));
        }
    }

    #[test]
    fn a_connection_aborted_chain_is_tagged_and_kept_whole() {
        let err = wrap(
            "Error reading data for partition system",
            io::Error::new(io::ErrorKind::ConnectionAborted, "device disconnected"),
        );
        let failure = WriteFailure::classify(&err);
        assert_eq!(
            failure.message,
            format!(
                "{TAG_QDL_DISCONNECTED} Error reading data for partition system: device disconnected"
            )
        );
        assert!(!failure.keep_armed);
        assert!(!failure.needs_provisioning);

        let bare = io::Error::new(io::ErrorKind::ConnectionAborted, "gone");
        assert!(WriteFailure::classify(&bare)
            .message
            .starts_with(TAG_QDL_DISCONNECTED));
    }

    #[test]
    fn send_and_zlp_fragments_are_disconnects() {
        for text in [
            "Error sending data for partition system",
            "Error sending ZLP",
        ] {
            let err = wrap(text, io::Error::other("boom"));
            let failure = WriteFailure::classify(&err);
            assert!(failure.message.starts_with(TAG_QDL_DISCONNECTED), "{text}");
            assert!(!failure.keep_armed);
            assert!(is_disconnect_panic(text));
        }
    }

    #[test]
    fn the_receive_fragment_tags_only_panic_text() {
        let err = wrap(
            "Got malformed data: {\"value\": \"Error receiving data\"}",
            io::Error::other("inner"),
        );
        let failure = WriteFailure::classify(&err);
        assert!(
            !failure.message.contains(TAG_QDL_DISCONNECTED),
            "{failure:?}"
        );
        assert!(!WriteFailure::classify(&msg("Error receiving data"))
            .message
            .contains(TAG_QDL_DISCONNECTED));

        assert!(is_disconnect_panic(
            "task 3 panicked with message \"Error receiving data: Kind(TimedOut)\""
        ));
    }

    #[test]
    fn a_plain_timeout_is_not_tagged() {
        let err = io::Error::new(io::ErrorKind::TimedOut, "timed out");
        let failure = WriteFailure::classify(&err);
        assert_eq!(failure.message, "timed out");
        assert!(!failure.keep_armed);
        assert!(!is_disconnect_panic("Error reading data for partition x"));
    }

    #[test]
    fn an_interrupted_io_error_is_not_a_cancel() {
        let err = wrap(
            "Error reading data for partition system",
            io::Error::new(io::ErrorKind::Interrupted, "transfer was cancelled"),
        );
        let failure = WriteFailure::classify(&err);
        assert!(!failure.message.starts_with(TAG_QDL_DISCONNECTED));
        assert!(!failure.message.contains(QDL_CANCELLED_ERROR));
        assert_eq!(
            settle(Err(failure.message.clone()), false),
            Err(failure.message)
        );
    }

    #[test]
    fn provisioning_only_for_a_bare_program_nak() {
        let nak = msg("<program> was NAKed. Did you set sector-size correctly?");
        let failure = WriteFailure::classify(&nak);
        assert!(failure.needs_provisioning);
        assert!(failure.keep_armed);

        let embedded = msg(
            "Got malformed data: {\"value\": \"<program> was NAKed. Did you set sector-size correctly?\"}",
        );
        assert!(!WriteFailure::classify(&embedded).needs_provisioning);

        let wrapped = wrap(
            "<program> was NAKed. Did you set sector-size correctly?",
            io::Error::other("inner"),
        );
        assert!(!WriteFailure::classify(&wrapped).needs_provisioning);
    }

    #[test]
    fn only_the_program_and_end_of_write_naks_keep_the_device_armed() {
        let end = WriteFailure::classify(&msg("Failed to complete 'write' op"));
        assert!(end.keep_armed);
        assert!(!end.needs_provisioning);

        for err in [
            msg("Wrote an unexpected number of bytes (12)"),
            msg("Failed to parse XML: bad"),
            wrap("Error sending ZLP", msg("Failed to complete 'write' op")),
        ] {
            assert!(!WriteFailure::classify(&err).keep_armed, "{err}");
        }
        assert!(WriteFailure::before_write("open failed".to_string()).keep_armed);
    }
}
