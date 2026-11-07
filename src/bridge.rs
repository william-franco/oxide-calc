//! Wires Slint callbacks to the view model and pushes state back into the window.
//! Calculations and history changes are also written to storage when one is available.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::engine::AngleMode;
use crate::settings;
use crate::storage::Storage;
use crate::viewmodel::CalcViewModel;
use crate::{AppWindow, HistoryItem, OxideTheme};

struct Bridge {
    vm: RefCell<CalcViewModel>,
    /// `None` when the database could not be opened: the calculator still works, without saving.
    storage: Option<Storage>,
    history: Rc<VecModel<HistoryItem>>,
    history_signature: RefCell<Option<(usize, Option<String>)>>,
}

impl Bridge {
    fn sync(&self, ui: &AppWindow) {
        let vm = self.vm.borrow();
        ui.set_expression(vm.expression().into());
        ui.set_secondary(vm.secondary_text().into());
        ui.set_has_error(vm.has_error());
        ui.set_degrees(vm.angle() == AngleMode::Degrees);

        // Rebuild the model only when the history actually changed.
        let signature = (vm.history().len(), vm.history().first().map(|e| e.expression.clone() + &e.result));
        if self.history_signature.borrow().as_ref() != Some(&signature) {
            self.history.set_vec(
                vm.history()
                    .iter()
                    .map(|entry| HistoryItem {
                        expression: entry.expression.as_str().into(),
                        result: entry.result.as_str().into(),
                    })
                    .collect::<Vec<_>>(),
            );
            *self.history_signature.borrow_mut() = Some(signature);
        }
    }

    /// Writes a failure to stderr. The calculator has no message area for storage errors.
    fn report(&self, what: &str, error: impl std::fmt::Display) {
        eprintln!("oxide-calc: não foi possível {what}: {error}");
    }
}

pub fn bind(ui: &AppWindow, storage: Option<Storage>) {
    if let Some(dark) = storage.as_ref().and_then(|s| settings::dark_theme(s).ok().flatten()) {
        ui.global::<OxideTheme>().set_dark(dark);
        ui.set_theme_saved(true);
    }

    let history = match storage.as_ref().map(Storage::load_history) {
        Some(Ok(entries)) => entries,
        Some(Err(e)) => {
            eprintln!("oxide-calc: não foi possível carregar o histórico: {e}");
            Vec::new()
        }
        None => Vec::new(),
    };

    let bridge = Rc::new(Bridge {
        vm: RefCell::new(CalcViewModel::with_history(history)),
        storage,
        history: Rc::new(VecModel::default()),
        history_signature: RefCell::new(None),
    });
    ui.set_history(ModelRc::from(bridge.history.clone()));
    bridge.sync(ui);

    {
        let bridge = bridge.clone();
        ui.on_dark_changed(move |dark: bool| {
            if let Some(storage) = &bridge.storage
                && let Err(e) = settings::set_dark_theme(storage, dark)
            {
                bridge.report("salvar a preferência de tema", e);
            }
        });
    }

    // Every callback mutates the view model, then re-renders.
    let action = |f: fn(&mut CalcViewModel)| {
        let bridge = bridge.clone();
        let weak = ui.as_weak();
        move || {
            f(&mut bridge.vm.borrow_mut());
            if let Some(ui) = weak.upgrade() {
                bridge.sync(&ui);
            }
        }
    };

    ui.on_backspace(action(CalcViewModel::backspace));
    ui.on_clear(action(CalcViewModel::clear));
    ui.on_toggle_sign(action(CalcViewModel::toggle_sign));
    ui.on_toggle_angle(action(CalcViewModel::toggle_angle));

    {
        let bridge = bridge.clone();
        let weak = ui.as_weak();
        ui.on_equals(move || {
            let entry = {
                let mut vm = bridge.vm.borrow_mut();
                vm.equals();
                vm.take_last_entry()
            };
            if let (Some(entry), Some(storage)) = (entry, &bridge.storage)
                && let Err(e) = storage.push_history(&entry)
            {
                bridge.report("salvar o cálculo no histórico", e);
            }
            if let Some(ui) = weak.upgrade() {
                bridge.sync(&ui);
            }
        });
    }
    {
        let bridge = bridge.clone();
        let weak = ui.as_weak();
        ui.on_clear_history(move || {
            bridge.vm.borrow_mut().clear_history();
            if let Some(storage) = &bridge.storage
                && let Err(e) = storage.clear_history()
            {
                bridge.report("limpar o histórico salvo", e);
            }
            if let Some(ui) = weak.upgrade() {
                bridge.sync(&ui);
            }
        });
    }

    {
        let bridge = bridge.clone();
        let weak = ui.as_weak();
        ui.on_key(move |token: SharedString| {
            bridge.vm.borrow_mut().input(&token);
            if let Some(ui) = weak.upgrade() {
                bridge.sync(&ui);
            }
        });
    }
    {
        let bridge = bridge.clone();
        let weak = ui.as_weak();
        ui.on_keyboard(move |text: SharedString| {
            let handled = bridge.vm.borrow_mut().handle_key(&text);
            if handled && let Some(ui) = weak.upgrade() {
                bridge.sync(&ui);
            }
            handled
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_reuse_history(move |index: i32| {
            if let Ok(index) = usize::try_from(index) {
                bridge.vm.borrow_mut().reuse_history(index);
            }
            if let Some(ui) = weak.upgrade() {
                bridge.sync(&ui);
            }
        });
    }
}
