use std::time::Duration;

use crate::{
    LanguagePair, SourceLanguage, TranslationController, TranslationMode, TranslationOperation,
};

/// The product delay for live translation after the last input change.
pub const LIVE_TRANSLATION_DEBOUNCE: Duration = Duration::from_millis(1_000);

/// Identifies one scheduled translation so replaced or cancelled timer
/// callbacks can be ignored safely.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DebounceTicket(u64);

/// The result of updating a pending live-translation timer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DebounceUpdate {
    /// The current pending timer already matches the controller state.
    Unchanged,
    /// Cancel this timer because its input is no longer eligible for live
    /// translation.
    Cancel { ticket: DebounceTicket },
    /// Replace the previous timer, if any, with a timer for the current input.
    Schedule {
        /// A previous timer to cancel before scheduling this one.
        cancel: Option<DebounceTicket>,
        /// The identifier to pass back when this timer fires.
        ticket: DebounceTicket,
        /// The configured debounce delay.
        delay: Duration,
    },
}

#[derive(Debug, Eq, PartialEq)]
struct PendingTranslation {
    ticket: DebounceTicket,
    input_text: String,
    language_pair: LanguagePair,
}

/// Coordinates debounce decisions without owning a clock or UI timer.
///
/// Call [`Self::update`] after input, language-pair, or mode changes. The
/// returned update tells a caller-provided scheduler which timer to cancel or
/// start. When its timer fires, pass the ticket to
/// [`Self::begin_if_pending`]; obsolete timers cannot start a translation.
#[derive(Debug)]
pub struct LiveTranslationDebouncer {
    delay: Duration,
    next_ticket: u64,
    pending: Option<PendingTranslation>,
}

impl Default for LiveTranslationDebouncer {
    fn default() -> Self {
        Self::new(LIVE_TRANSLATION_DEBOUNCE)
    }
}

impl LiveTranslationDebouncer {
    /// Creates a debouncer with a caller-selected delay.
    pub fn new(delay: Duration) -> Self {
        Self {
            delay,
            next_ticket: 0,
            pending: None,
        }
    }

    /// Returns the configured delay.
    pub fn delay(&self) -> Duration {
        self.delay
    }

    /// Reconciles the pending timer with the current controller state.
    ///
    /// Live translation is scheduled only for nonblank input and a valid
    /// source/target combination. Explicitly selecting the same language for
    /// source and target is treated as invalid; auto-detection remains valid.
    pub fn update(&mut self, controller: &TranslationController) -> DebounceUpdate {
        if !is_schedulable(controller) {
            return self.cancel();
        }

        if let Some(pending) = &self.pending
            && pending.input_text == controller.input_text()
            && pending.language_pair == *controller.language_pair()
        {
            return DebounceUpdate::Unchanged;
        }

        let cancel = self.pending.take().map(|pending| pending.ticket);
        let ticket = self.next_ticket();
        self.pending = Some(PendingTranslation {
            ticket,
            input_text: controller.input_text().to_owned(),
            language_pair: controller.language_pair().clone(),
        });

        DebounceUpdate::Schedule {
            cancel,
            ticket,
            delay: self.delay,
        }
    }

    /// Cancels the pending timer, if there is one.
    pub fn cancel(&mut self) -> DebounceUpdate {
        match self.pending.take() {
            Some(pending) => DebounceUpdate::Cancel {
                ticket: pending.ticket,
            },
            None => DebounceUpdate::Unchanged,
        }
    }

    /// Starts translation if `ticket` is still current and the controller
    /// still has the same eligible live-translation input.
    ///
    /// A timer callback can safely call this even after cancellation or
    /// replacement. It returns `None` for stale tickets, manual mode, blank
    /// input, or an invalid language pair.
    pub fn begin_if_pending(
        &mut self,
        ticket: DebounceTicket,
        controller: &mut TranslationController,
    ) -> Option<TranslationOperation> {
        let pending = self.pending.as_ref()?;
        if pending.ticket != ticket {
            return None;
        }

        if pending.input_text != controller.input_text()
            || pending.language_pair != *controller.language_pair()
            || !is_schedulable(controller)
        {
            self.pending = None;
            return None;
        }

        self.pending = None;
        controller.begin_translation()
    }

    fn next_ticket(&mut self) -> DebounceTicket {
        self.next_ticket = self.next_ticket.wrapping_add(1);
        DebounceTicket(self.next_ticket)
    }
}

fn is_schedulable(controller: &TranslationController) -> bool {
    if controller.mode() != TranslationMode::Live || controller.input_text().trim().is_empty() {
        return false;
    }

    match controller.language_pair().source_language() {
        SourceLanguage::AutoDetect => true,
        SourceLanguage::Specific(source) => source != controller.language_pair().target_language(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{LanguageCode, LanguagePair, SourceLanguage, TranslationController};

    use super::{
        DebounceTicket, DebounceUpdate, LIVE_TRANSLATION_DEBOUNCE, LiveTranslationDebouncer,
    };

    fn code(value: &str) -> LanguageCode {
        LanguageCode::new(value).expect("valid language code")
    }

    fn pair(source: SourceLanguage, target: &str) -> LanguagePair {
        LanguagePair::new(source, code(target)).expect("supported language pair")
    }

    fn live_controller(input: &str, language_pair: LanguagePair) -> TranslationController {
        let mut controller =
            TranslationController::new(language_pair, crate::TranslationMode::Live);
        controller.set_input_text(input);
        controller
    }

    fn scheduled(update: DebounceUpdate) -> (Option<DebounceTicket>, DebounceTicket, Duration) {
        match update {
            DebounceUpdate::Schedule {
                cancel,
                ticket,
                delay,
            } => (cancel, ticket, delay),
            other => panic!("expected a scheduled timer, got {other:?}"),
        }
    }

    #[test]
    fn default_uses_the_product_debounce_delay() {
        let debouncer = LiveTranslationDebouncer::default();

        assert_eq!(LIVE_TRANSLATION_DEBOUNCE, Duration::from_millis(1_000));
        assert_eq!(debouncer.delay(), LIVE_TRANSLATION_DEBOUNCE);
    }

    #[test]
    fn configured_delay_is_returned_to_the_injected_scheduler() {
        let delay = Duration::from_millis(250);
        let mut debouncer = LiveTranslationDebouncer::new(delay);
        let controller = live_controller("hello", pair(SourceLanguage::AutoDetect, "en"));

        let (_, _, scheduled_delay) = scheduled(debouncer.update(&controller));

        assert_eq!(scheduled_delay, delay);
    }

    #[test]
    fn input_changes_replace_the_timer_and_ignore_the_old_callback() {
        let mut debouncer = LiveTranslationDebouncer::default();
        let mut controller = live_controller("first", pair(SourceLanguage::AutoDetect, "en"));
        let (_, first_ticket, _) = scheduled(debouncer.update(&controller));

        controller.set_input_text("latest");
        let (cancelled, latest_ticket, _) = scheduled(debouncer.update(&controller));

        assert_eq!(cancelled, Some(first_ticket));
        assert!(
            debouncer
                .begin_if_pending(first_ticket, &mut controller)
                .is_none()
        );
        assert_eq!(
            debouncer
                .begin_if_pending(latest_ticket, &mut controller)
                .expect("latest timer starts translation")
                .request()
                .text,
            "latest"
        );
    }

    #[test]
    fn cancellation_invalidates_pending_timer_and_is_idempotent() {
        let mut debouncer = LiveTranslationDebouncer::default();
        let mut controller = live_controller("hello", pair(SourceLanguage::AutoDetect, "en"));
        let (_, ticket, _) = scheduled(debouncer.update(&controller));

        assert_eq!(debouncer.cancel(), DebounceUpdate::Cancel { ticket });
        assert_eq!(debouncer.cancel(), DebounceUpdate::Unchanged);
        assert!(
            debouncer
                .begin_if_pending(ticket, &mut controller)
                .is_none()
        );
    }

    #[test]
    fn manual_mode_does_not_schedule_and_cancels_live_timer() {
        let mut debouncer = LiveTranslationDebouncer::default();
        let mut controller = live_controller("hello", pair(SourceLanguage::AutoDetect, "en"));
        let (_, ticket, _) = scheduled(debouncer.update(&controller));

        controller.set_mode(crate::TranslationMode::Manual);
        assert_eq!(
            debouncer.update(&controller),
            DebounceUpdate::Cancel { ticket }
        );

        controller.set_mode(crate::TranslationMode::Live);
        let (_, _, _) = scheduled(debouncer.update(&controller));
    }

    #[test]
    fn blank_input_does_not_schedule_or_keep_old_work() {
        let mut debouncer = LiveTranslationDebouncer::default();
        let mut controller = live_controller("hello", pair(SourceLanguage::AutoDetect, "en"));
        let (_, ticket, _) = scheduled(debouncer.update(&controller));

        controller.set_input_text(" \n\t ");
        assert_eq!(
            debouncer.update(&controller),
            DebounceUpdate::Cancel { ticket }
        );
    }

    #[test]
    fn identical_explicit_languages_do_not_schedule() {
        let mut debouncer = LiveTranslationDebouncer::default();
        let controller = live_controller("hello", pair(SourceLanguage::Specific(code("en")), "en"));

        assert_eq!(debouncer.update(&controller), DebounceUpdate::Unchanged);
    }

    #[test]
    fn invalid_pair_selected_after_scheduling_cannot_start_live_request() {
        let mut debouncer = LiveTranslationDebouncer::default();
        let mut controller = live_controller("hello", pair(SourceLanguage::AutoDetect, "en"));
        let (_, ticket, _) = scheduled(debouncer.update(&controller));

        controller.set_language_pair(pair(SourceLanguage::Specific(code("en")), "en"));

        assert!(
            debouncer
                .begin_if_pending(ticket, &mut controller)
                .is_none()
        );
        assert_eq!(controller.state(), &crate::TranslationState::Idle);
    }

    #[test]
    fn auto_detect_source_is_valid_and_pending_work_is_one_shot() {
        let mut debouncer = LiveTranslationDebouncer::default();
        let mut controller = live_controller("hello", pair(SourceLanguage::AutoDetect, "en"));
        let (_, ticket, _) = scheduled(debouncer.update(&controller));

        assert_eq!(
            debouncer
                .begin_if_pending(ticket, &mut controller)
                .expect("auto detection can be translated")
                .request()
                .text,
            "hello"
        );
        assert!(
            debouncer
                .begin_if_pending(ticket, &mut controller)
                .is_none()
        );
    }
}
