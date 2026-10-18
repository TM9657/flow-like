use std::cell::Cell;

use crate::SolveApp;

thread_local! {
    static PREVIEW: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn read_only() -> bool {
    PREVIEW.get()
}

fn clear_input(ctx: &egui::Context) {
    ctx.input_mut(|input| {
        input.events.clear();
        input.raw.events.clear();
        input.pointer = Default::default();
        input.modifiers = egui::Modifiers::NONE;
        input.keys_down.clear();
        input.smooth_scroll_delta = egui::Vec2::ZERO;
    });
}

impl SolveApp {
    pub fn host_preview(&mut self, ui: &mut egui::Ui) {
        PREVIEW.set(true);
        crate::viewport::show(self, ui);
        self.resolve_fit(ui.ctx());
        PREVIEW.set(false);
    }
}

pub(crate) fn overlays(ui: &mut egui::Ui, show: impl FnOnce(&mut egui::Ui)) {
    if !read_only() {
        show(ui);
        return;
    }
    let input = ui.ctx().input(Clone::clone);
    clear_input(ui.ctx());
    ui.add_enabled_ui(false, show);
    ui.ctx().input_mut(|current| *current = input);
}
