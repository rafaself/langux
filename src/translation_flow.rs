use gtk::prelude::*;
use gtk::{ApplicationWindow, DropDown, TextView, glib};
use langux_core::{
    LanguagePair, LiveTranslationDebouncer, SourceLanguage, TranslationController,
    TranslationError, TranslationMode, TranslationOperation, TranslationProvider,
};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;

use crate::language_selection::language_pair;
use crate::translation_view::TranslationView;

struct TranslationUiState {
    controller: TranslationController,
    debouncer: LiveTranslationDebouncer,
    pending_timer: Option<(langux_core::DebounceTicket, glib::SourceId)>,
    provider: Arc<dyn TranslationProvider>,
    view: TranslationView,
    closed: bool,
}

pub fn connect(
    window: &ApplicationWindow,
    input_view: &TextView,
    source_dropdown: &DropDown,
    target_dropdown: &DropDown,
    initial_pair: LanguagePair,
    view: TranslationView,
    provider: Arc<dyn TranslationProvider>,
) {
    let state = Rc::new(RefCell::new(TranslationUiState {
        controller: TranslationController::new(initial_pair, TranslationMode::Live),
        debouncer: LiveTranslationDebouncer::default(),
        pending_timer: None,
        provider,
        view,
        closed: false,
    }));

    let input_buffer = input_view.buffer();
    let state_for_input = Rc::downgrade(&state);
    input_buffer.connect_changed(move |buffer| {
        let Some(state) = state_for_input.upgrade() else {
            return;
        };
        let (start, end) = buffer.bounds();
        let input_text = buffer.text(&start, &end, true).to_string();
        if state.borrow().closed {
            return;
        }
        state.borrow_mut().controller.set_input_text(input_text);
        render(&state);
        reconcile_live_timer(&state);
    });

    let target_weak = target_dropdown.downgrade();
    let state_for_source = Rc::downgrade(&state);
    source_dropdown.connect_selected_notify(move |source| {
        let Some(state) = state_for_source.upgrade() else {
            return;
        };
        let Some(target) = target_weak.upgrade() else {
            return;
        };
        update_language_pair(&state, source.selected(), target.selected());
    });

    let source_weak = source_dropdown.downgrade();
    let state_for_target = Rc::downgrade(&state);
    target_dropdown.connect_selected_notify(move |target| {
        let Some(state) = state_for_target.upgrade() else {
            return;
        };
        let Some(source) = source_weak.upgrade() else {
            return;
        };
        update_language_pair(&state, source.selected(), target.selected());
    });

    // Keep the controller alive for as long as the window is alive. The state
    // owns child widgets but not the window, so this does not form a cycle.
    let state_for_close = Rc::clone(&state);
    window.connect_close_request(move |_| {
        cancel_pending_timer(&state_for_close, None);
        let mut state = state_for_close.borrow_mut();
        state.closed = true;
        state.controller.cancel_translation();
        glib::Propagation::Proceed
    });

    render(&state);
}

fn update_language_pair(
    state: &Rc<RefCell<TranslationUiState>>,
    source_index: u32,
    target_index: u32,
) {
    let Some(pair) = language_pair(source_index, target_index) else {
        return;
    };
    if state.borrow().closed {
        return;
    }
    state.borrow_mut().controller.set_language_pair(pair);
    render(state);
    reconcile_live_timer(state);
}

fn reconcile_live_timer(state: &Rc<RefCell<TranslationUiState>>) {
    let update = {
        let mut state = state.borrow_mut();
        let TranslationUiState {
            controller,
            debouncer,
            ..
        } = &mut *state;
        debouncer.update(controller)
    };

    match update {
        langux_core::DebounceUpdate::Unchanged => {}
        langux_core::DebounceUpdate::Cancel { ticket } => {
            cancel_pending_timer(state, Some(ticket));
        }
        langux_core::DebounceUpdate::Schedule { ticket, delay, .. } => {
            cancel_pending_timer(state, None);
            let state_weak = Rc::downgrade(state);
            let source_id = glib::timeout_add_local_once(delay, move || {
                let Some(state) = state_weak.upgrade() else {
                    return;
                };
                let operation = {
                    let mut state = state.borrow_mut();
                    state.pending_timer = None;
                    if state.closed {
                        return;
                    }
                    let TranslationUiState {
                        controller,
                        debouncer,
                        ..
                    } = &mut *state;
                    debouncer.begin_if_pending(ticket, controller)
                };
                render(&state);
                if let Some(operation) = operation {
                    start_translation(&state, operation);
                }
            });

            let mut state = state.borrow_mut();
            if state.closed {
                source_id.remove();
            } else {
                state.pending_timer = Some((ticket, source_id));
            }
        }
    }
}

fn cancel_pending_timer(
    state: &Rc<RefCell<TranslationUiState>>,
    ticket: Option<langux_core::DebounceTicket>,
) {
    let source_id = {
        let mut state = state.borrow_mut();
        if state
            .pending_timer
            .as_ref()
            .is_some_and(|(current_ticket, _)| {
                ticket.is_none_or(|ticket| ticket == *current_ticket)
            })
        {
            state.pending_timer.take().map(|(_, source_id)| source_id)
        } else {
            None
        }
    };
    if let Some(source_id) = source_id {
        source_id.remove();
    }
}

fn start_translation(state: &Rc<RefCell<TranslationUiState>>, operation: TranslationOperation) {
    let (provider, closed) = {
        let state = state.borrow();
        (Arc::clone(&state.provider), state.closed)
    };
    if closed {
        return;
    }

    let worker_operation = operation.clone();
    let worker = gtk::gio::spawn_blocking(move || {
        let cancellation = worker_operation.cancellation_token();
        provider.translate(worker_operation.request(), &cancellation)
    });
    let state_weak: Weak<RefCell<TranslationUiState>> = Rc::downgrade(state);
    glib::MainContext::default().spawn_local(async move {
        let outcome = worker
            .await
            .unwrap_or(Err(TranslationError::ProviderFailure));
        let Some(state) = state_weak.upgrade() else {
            return;
        };
        let accepted = {
            let mut state = state.borrow_mut();
            !state.closed && state.controller.finish_translation(&operation, outcome)
        };
        if accepted {
            render(&state);
        }
    });
}

fn render(state: &Rc<RefCell<TranslationUiState>>) {
    let state = state.borrow();
    let pair = state.controller.language_pair();
    let invalid_language_pair = !state.controller.input_text().trim().is_empty()
        && matches!(pair.source_language(), SourceLanguage::Specific(source) if source == pair.target_language());
    state
        .view
        .render(state.controller.state(), invalid_language_pair);
}
