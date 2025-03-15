use std::path::PathBuf;

use catppuccin_egui::MOCHA;
use egui::{
    Align, Button, CentralPanel, Context, Frame, Label, Layout, Modifiers, OpenUrl, RichText,
    ScrollArea, TopBottomPanel, Ui,
};
use egui_extras::{Size, StripBuilder};

use crate::{spawn_worker, Message, Task};

const MAX_WORKERS: usize = 16;

struct BacklogTask {
    path: PathBuf,
    modifiers: Modifiers,
}

#[derive(Default)]
pub struct MainState {
    messages: Vec<Message>,
    tasks: Vec<Task>,
    backlog: Vec<BacklogTask>,
}

impl MainState {
    pub fn show(&mut self, ctx: &Context) {
        self.update_backlog();

        TopBottomPanel::top("global_top_panel")
            .show_separator_line(true)
            .show_separator_line(false)
            .frame(Frame::side_top_panel(&ctx.style()).fill(MOCHA.crust))
            .show(ctx, |ui| {
                StripBuilder::new(ui)
                    .size(Size::exact(300.))
                    .size(Size::remainder())
                    .size(Size::exact(200.))
                    .horizontal(|mut strip| {
                        strip.cell(|ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .add(Button::new("").fill(MOCHA.crust))
                                    .on_hover_text("Open GitHub Repository")
                                    .clicked()
                                {
                                    ctx.open_url(OpenUrl {
                                        url: "https://github.com/thane98/fe9cmp".into(),
                                        new_tab: true,
                                    });
                                }
                                if ui
                                    .add(Button::new("🗑").fill(MOCHA.crust))
                                    .on_hover_text("Clear Messages")
                                    .clicked()
                                {
                                    self.messages.clear();
                                }
                            });
                        });
                        strip.cell(|_| {});
                        strip.cell(|ui| {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let pending_tasks = self.tasks.len() + self.backlog.len();
                                if pending_tasks > 0 {
                                    let noun = if pending_tasks == 1 { "task" } else { "tasks" };
                                    ui.label(format!("{} {} remaining...", pending_tasks, noun));
                                    ui.spinner();
                                } else {
                                    ui.label("No pending tasks.");
                                }
                            });
                        });
                    });
            });

        CentralPanel::default().show(ctx, |ui| {
            ui.collapsing("Instructions", |ui| {
                bullet(ui, "Drop a .cmp file to extract it to a folder");
                bullet(ui, "Drop a folder to pack it into a .cmp file");
                bullet(ui, "Drop a .m file to extract it to a .yml file");
                bullet(ui, "Drop a .yml file to pack it into a .m file");
                bullet(ui, "Drop a .bin file to extract it to a text file.");
                bullet(ui, "Drop a .cms file to decompress to .bin and try to extract as text");
                bullet(ui, "Hold cmd / ctrl while dropping to force LZ10 compressing the file (saves as .cms)");
                bullet(ui, "Hold shift while dropping to force LZ10 decompressing the file (saves as .bin");
            });

            Frame::central_panel(&ctx.style()).inner_margin(10.).fill(MOCHA.crust).corner_radius(8.).show(ui, |ui| {
                ScrollArea::vertical().auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
                    for message in &self.messages {
                        match message {
                            Message::Success(success_message) => {
                                ui.monospace(success_message);
                            }
                            Message::Error(error_message) => {
                                let text = RichText::new(error_message).monospace().color(ui.visuals().error_fg_color);
                                ui.add(Label::new(text));
                            }
                        }
                    }
                });
            });

            ui.input(|input| {
                for file in &input.raw.dropped_files {
                    if let Some(path) = file.path.clone() {
                        self.enqueue(path, input.modifiers);
                    }
                }
            });
        });
    }

    fn enqueue(&mut self, path: PathBuf, modifiers: Modifiers) {
        if self.tasks.len() < MAX_WORKERS {
            self.tasks.push(spawn_worker(path, modifiers));
        } else {
            self.backlog.push(BacklogTask { path, modifiers });
        }
    }

    fn update_backlog(&mut self) {
        self.tasks.retain_mut(|task| {
            if let Some(messages) = task.poll() {
                self.messages.extend(messages);
            }
            !task.done
        });

        let enqueue_count = MAX_WORKERS
            .min(self.backlog.len())
            .saturating_sub(self.tasks.len());
        let tasks_to_enqueue: Vec<BacklogTask> = self.backlog.drain(0..enqueue_count).collect();
        for task in tasks_to_enqueue {
            self.tasks.push(spawn_worker(task.path, task.modifiers));
        }
    }
}

fn bullet(ui: &mut Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.label("•");
        ui.label(text);
    });
}
