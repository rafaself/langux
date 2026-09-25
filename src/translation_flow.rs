use gtk::gio::prelude::*;
use gtk::prelude::*;
use gtk::{ApplicationWindow, DropDown, EventControllerKey, PropagationPhase, TextView, gdk, glib};
use langux_core::{
    LanguagePair, LiveTranslationDebouncer, SourceLanguage, TranslationController,
    TranslationError, TranslationMode, TranslationOperation, TranslationProvider,
};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;

use crate::input_key_behavior::{self, InputKey, KeyAction};
use crate::language_controls::LanguageControls;
use crate::language_selection::language_pair;
use crate::translation_view::TranslationView;

struct TranslationUiState {
    controller: TranslationController,
    debouncer: LiveTranslationDebouncer,
    pending_timer: Option<(langux_core::DebounceTicket, glib::SourceId)>,
    provider: Arc<dyn TranslationProvider>,
    settings: gtk::gio::Settings,
    settings_changed_ids: Vec<glib::SignalHandlerId>,
    view: TranslationView,
    closed: bool,
}

pub fn connect(
    window: &ApplicationWindow,
    input_view: &TextView,
    language_controls: &LanguageControls,
    initial_pair: LanguagePair,
    settings: gtk::gio::Settings,
    view: TranslationView,
    provider: Arc<dyn TranslationProvider>,
) {
    let state = Rc::new(RefCell::new(TranslationUiState {
        controller: TranslationController::with_cache_capacity(
            initial_pair,
            crate::settings::translation_mode(&settings),
            crate::settings::cache_capacity(&settings),
        ),
        debouncer: LiveTranslationDebouncer::default(),
        pending_timer: None,
        provider,
        settings,
        settings_changed_ids: Vec::new(),
        view,
        closed: false,
    }));

    connect_escape_key(window);
    connect_translation_keys(input_view, &state);

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

    let target_weak = language_controls.target_dropdown.downgrade();
    let state_for_source = Rc::downgrade(&state);
    language_controls
        .source_dropdown
        .connect_selected_notify(move |source| {
            let Some(state) = state_for_source.upgrade() else {
                return;
            };
            let Some(target) = target_weak.upgrade() else {
                return;
            };
            update_language_pair(&state, source.selected(), target.selected());
        });

    let source_weak = language_controls.source_dropdown.downgrade();
    let state_for_target = Rc::downgrade(&state);
    language_controls
        .target_dropdown
        .connect_selected_notify(move |target| {
            let Some(state) = state_for_target.upgrade() else {
                return;
            };
            let Some(source) = source_weak.upgrade() else {
                return;
            };
            update_language_pair(&state, source.selected(), target.selected());
        });

    connect_settings_updates(
        &state,
        &language_controls.source_dropdown,
        &language_controls.target_dropdown,
    );

    // Keep the controller alive for as long as the window is alive. The state
    // owns child widgets but not the window, so this does not form a cycle.
    let state_for_close = Rc::clone(&state);
    window.connect_close_request(move |_| {
        cancel_pending_timer(&state_for_close, None);
        let (settings, handlers) = {
            let mut state = state_for_close.borrow_mut();
            state.closed = true;
            state.controller.cancel_translation();
            (
                state.settings.clone(),
                std::mem::take(&mut state.settings_changed_ids),
            )
        };
        for handler in handlers {
            settings.disconnect(handler);
        }
        glib::Propagation::Proceed
    });

    render(&state);
}

fn connect_escape_key(window: &ApplicationWindow) {
    let key_controller = EventControllerKey::new();
    key_controller.set_propagation_phase(PropagationPhase::Capture);
    let window_weak = window.downgrade();
    key_controller.connect_key_pressed(move |_, key, _, modifiers| {
        let action = input_key_behavior::action(
            false,
            input_key(key),
            modifiers.contains(gdk::ModifierType::CONTROL_MASK),
            modifiers.contains(gdk::ModifierType::SHIFT_MASK),
            has_other_modifier(modifiers),
        );
        if action == KeyAction::Close {
            if let Some(window) = window_weak.upgrade() {
                window.close();
                return glib::Propagation::Stop;
            }
        }
        glib::Propagation::Proceed
    });
    window.add_controller(key_controller);
}

fn connect_translation_keys(input_view: &TextView, state: &Rc<RefCell<TranslationUiState>>) {
    let key_controller = EventControllerKey::new();
    key_controller.set_propagation_phase(PropagationPhase::Capture);
    let state_weak = Rc::downgrade(state);
    key_controller.connect_key_pressed(move |_, key, _, modifiers| {
        let Some(state) = state_weak.upgrade() else {
            return glib::Propagation::Proceed;
        };
        let manual_mode = state.borrow().controller.mode() == TranslationMode::Manual;
        let action = input_key_behavior::action(
            manual_mode,
            input_key(key),
            modifiers.contains(gdk::ModifierType::CONTROL_MASK),
            modifiers.contains(gdk::ModifierType::SHIFT_MASK),
            has_other_modifier(modifiers),
        );
        if action == KeyAction::Translate {
            translate_now(&state);
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    input_view.add_controller(key_controller);
}

fn input_key(key: gdk::Key) -> InputKey {
    match key {
        gdk::Key::Escape => InputKey::Escape,
        gdk::Key::Return | gdk::Key::KP_Enter => InputKey::Enter,
        _ => InputKey::Other,
    }
}

fn has_other_modifier(modifiers: gdk::ModifierType) -> bool {
    modifiers.intersects(
        gdk::ModifierType::ALT_MASK
            | gdk::ModifierType::SUPER_MASK
            | gdk::ModifierType::HYPER_MASK
            | gdk::ModifierType::META_MASK,
    )
}

fn translate_now(state: &Rc<RefCell<TranslationUiState>>) {
    cancel_pending_timer(state, None);
    let operation = {
        let mut state = state.borrow_mut();
        state.debouncer.cancel();
        if state.closed || invalid_language_pair(&state.controller) {
            return;
        }
        state.controller.begin_translation()
    };
    render(state);
    if let Some(operation) = operation {
        start_translation(state, operation);
    }
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
    let settings = state.borrow().settings.clone();
    let _ = crate::settings::save_language_pair(&settings, &pair);
    state.borrow_mut().controller.set_language_pair(pair);
    render(state);
    reconcile_live_timer(state);
}

fn connect_settings_updates(
    state: &Rc<RefCell<TranslationUiState>>,
    source_dropdown: &DropDown,
    target_dropdown: &DropDown,
) {
    let settings = state.borrow().settings.clone();

    let state_weak = Rc::downgrade(state);
    let source_weak = source_dropdown.downgrade();
    let target_weak = target_dropdown.downgrade();
    let language_changed = settings.connect_changed(
        Some(crate::settings::SOURCE_LANGUAGE_KEY),
        move |settings, _| {
            sync_language_controls(&state_weak, &source_weak, &target_weak, settings);
        },
    );

    let state_weak = Rc::downgrade(state);
    let source_weak = source_dropdown.downgrade();
    let target_weak = target_dropdown.downgrade();
    let target_language_changed = settings.connect_changed(
        Some(crate::settings::TARGET_LANGUAGE_KEY),
        move |settings, _| {
            sync_language_controls(&state_weak, &source_weak, &target_weak, settings);
        },
    );

    let state_weak = Rc::downgrade(state);
    let mode_changed = settings.connect_changed(
        Some(crate::settings::LIVE_TRANSLATION_KEY),
        move |settings, _| {
            let Some(state) = state_weak.upgrade() else {
                return;
            };
            if state.borrow().closed {
                return;
            }
            state
                .borrow_mut()
                .controller
                .set_mode(crate::settings::translation_mode(settings));
            render(&state);
            reconcile_live_timer(&state);
        },
    );

    let state_weak = Rc::downgrade(state);
    let cache_enabled_changed = settings.connect_changed(
        Some(crate::settings::CACHE_ENABLED_KEY),
        move |settings, _| {
            let Some(state) = state_weak.upgrade() else {
                return;
            };
            if state.borrow().closed {
                return;
            }
            state
                .borrow_mut()
                .controller
                .resize_translation_cache(crate::settings::cache_capacity(settings));
        },
    );

    let state_weak = Rc::downgrade(state);
    let cache_capacity_changed = settings.connect_changed(
        Some(crate::settings::CACHE_CAPACITY_KEY),
        move |settings, _| {
            let Some(state) = state_weak.upgrade() else {
                return;
            };
            if state.borrow().closed {
                return;
            }
            state
                .borrow_mut()
                .controller
                .resize_translation_cache(crate::settings::cache_capacity(settings));
        },
    );

    state.borrow_mut().settings_changed_ids.extend([
        language_changed,
        target_language_changed,
        mode_changed,
        cache_enabled_changed,
        cache_capacity_changed,
    ]);
}

fn sync_language_controls(
    state: &Weak<RefCell<TranslationUiState>>,
    source: &glib::WeakRef<DropDown>,
    target: &glib::WeakRef<DropDown>,
    settings: &gtk::gio::Settings,
) {
    let Some(state) = state.upgrade() else {
        return;
    };
    if state.borrow().closed {
        return;
    }
    let Some(source) = source.upgrade() else {
        return;
    };
    let Some(target) = target.upgrade() else {
        return;
    };
    let pair = crate::settings::language_pair(settings);
    if let Some((source_index, target_index)) = crate::language_selection::language_indices(&pair) {
        if source.selected() != source_index {
            source.set_selected(source_index);
        }
        if target.selected() != target_index {
            target.set_selected(target_index);
        }
    }
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
                    if invalid_language_pair(&state.controller) {
                        state.debouncer.cancel();
                        None
                    } else {
                        let TranslationUiState {
                            controller,
                            debouncer,
                            ..
                        } = &mut *state;
                        debouncer.begin_if_pending(ticket, controller)
                    }
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
    state.view.render(
        state.controller.state(),
        invalid_language_pair(&state.controller),
    );
}

fn invalid_language_pair(controller: &TranslationController) -> bool {
    !controller.input_text().trim().is_empty()
        && matches!(
            controller.language_pair().source_language(),
            SourceLanguage::Specific(source) if source == controller.language_pair().target_language()
        )
}
