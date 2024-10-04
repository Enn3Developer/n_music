use crate::storage::Storage;
use eframe::egui::{Context, ViewportCommand, Visuals};
use eframe::{egui, Frame};
use std::rc::Rc;
use std::sync::Mutex;

pub struct InitApp {
    pub storage: Rc<Mutex<Storage>>,
}

impl InitApp {
    pub fn new(storage: Rc<Mutex<Storage>>) -> Self {
        Self { storage }
    }
}

impl eframe::App for InitApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        ctx.set_visuals(Visuals::dark());
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.add_space(15.0);
                ui.label("Import playlist to continue");
                if ui.button("Import").clicked() {
                    let file_dialog = rfd::FileDialog::new();
                    if let Some(path) = file_dialog.pick_folder() {
                        self.storage.lock().unwrap().path = path.to_str().unwrap().to_string();
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                }
            });
        });
    }
}
