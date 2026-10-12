//! Wires Slint callbacks to the view model and pushes state back into the window.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::engine::AngleMode;
use crate::viewmodel::CalcViewModel;
use crate::{AppWindow, HistoryItem};

struct Bridge {
    vm: RefCell<CalcViewModel>,
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
}

pub fn bind(ui: &AppWindow) {
    let bridge = Rc::new(Bridge {
        vm: RefCell::new(CalcViewModel::new()),
        history: Rc::new(VecModel::default()),
        history_signature: RefCell::new(None),
    });
    ui.set_history(ModelRc::from(bridge.history.clone()));
    bridge.sync(ui);

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
    ui.on_equals(action(CalcViewModel::equals));
    ui.on_clear_history(action(CalcViewModel::clear_history));
    ui.on_toggle_angle(action(CalcViewModel::toggle_angle));

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
