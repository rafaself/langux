use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::{TranslationError, TranslationRequest, TranslationResult};

/// Shared cooperative cancellation signal for one translation operation.
///
/// Providers can clone this token when work moves to a worker thread. The
/// controller marks it cancelled when the operation is cancelled,
/// superseded, or the controller is dropped.
#[derive(Clone, Debug)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub(crate) fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns whether the controller has cancelled this operation.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub(crate) fn belongs_to_same_operation(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.cancelled, &other.cancelled)
    }
}

/// Executes translation requests without exposing UI or provider-specific data.
///
/// Implementations receive only normalized Langux domain types. They should
/// check cancellation before starting work and while waiting or processing a
/// response when their transport allows it. A cancelled implementation should
/// stop promptly and return [`TranslationError::Cancelled`]. Some blocking
/// transports cannot abort an in-flight request; in that case, the controller
/// still rejects its completion after cancellation or supersession. Callers
/// should run blocking providers away from the UI thread.
pub trait TranslationProvider: Send + Sync {
    /// Translates one domain request, observing the operation's shared signal.
    fn translate(
        &self,
        request: &TranslationRequest,
        cancellation: &CancellationToken,
    ) -> Result<TranslationResult, TranslationError>;
}
