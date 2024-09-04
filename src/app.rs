use egui::{ScrollArea, Ui};
use egui_extras::{Column, TableBuilder};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct ThisApp {
    table_data: Vec<Vec<String>>,
    num_rows: usize,
    num_columns: usize,
    selected_rows: Vec<bool>,       // Add this line
    show_confirmation_dialog: bool, // Add this line
}

impl Default for ThisApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            table_data: vec![
                vec![
                    "Column 1".to_owned(),
                    "Column 2".to_owned(),
                    "Column 3".to_owned(),
                ],
                vec![
                    "Data 1".to_owned(),
                    "Data 2".to_owned(),
                    "Data 3".to_owned(),
                ],
            ],
            num_rows: 2,
            num_columns: 3,
            selected_rows: vec![false, false], // Add this line
            show_confirmation_dialog: false,   // Add this line
        }
    }
}

impl ThisApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for ThisApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                // Add button to add new rows
                if ui.button("➕ Add Row").clicked() {
                    self.add_row();
                }

                // Add Delete Selected Rows button
                if ui.button("🗑 Delete Selected Rows").clicked() {
                    self.delete_selected_rows(ctx);
                }

                // egui::widgets::global_dark_light_mode_buttons(ui);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::widgets::global_dark_light_mode_buttons(ui);
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.table_ui(ui);

            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                ui.add(egui::github_link_file!(
                    "https://github.com/Ashu999/web-value-tracker/blob/main/",
                    "Source code."
                ));
                egui::warn_if_debug_build(ui);
            });
        });

        if self.show_confirmation_dialog {
            let selected_count = self
                .selected_rows
                .iter()
                .filter(|&&selected| selected)
                .count();
            egui::Window::new("Confirm Deletion")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Are you sure you want to delete {} selected row(s)?",
                        selected_count
                    ));
                    ui.horizontal(|ui| {
                        if ui.button("Yes").clicked() {
                            self.perform_delete();
                            self.show_confirmation_dialog = false;
                        }
                        if ui.button("No").clicked() {
                            self.show_confirmation_dialog = false;
                        }
                    });
                });
        }
    }
}

impl ThisApp {
    fn add_row(&mut self) {
        let new_row = (0..self.num_columns)
            .map(|i| format!("New {}", i + 1))
            .collect();
        self.table_data.push(new_row);
        self.num_rows += 1;
        self.selected_rows.push(false); // Add this line
    }

    fn table_ui(&mut self, ui: &mut Ui) {
        ScrollArea::horizontal().show(ui, |ui| {
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .columns(Column::auto().at_least(30.0), 1) // Add checkbox column
                .columns(Column::auto().at_least(100.0), self.num_columns)
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("Select");
                    });
                    for col in 0..self.num_columns {
                        header.col(|ui| {
                            ui.strong(&self.table_data[0][col]);
                        });
                    }
                })
                .body(|mut body| {
                    for row_index in 1..self.table_data.len() {
                        let row_is_selected = self.selected_rows[row_index];
                        body.row(18.0, |mut row| {
                            row.col(|ui| {
                                if ui
                                    .checkbox(&mut self.selected_rows[row_index], "")
                                    .changed()
                                {
                                    ui.ctx().request_repaint(); // Ensure UI updates immediately
                                }
                            });
                            for col in 0..self.num_columns {
                                row.col(|ui| {
                                    let text_color = if row_is_selected {
                                        ui.ctx().style().visuals.strong_text_color()
                                    } else {
                                        ui.ctx().style().visuals.text_color()
                                    };
                                    ui.colored_label(text_color, &self.table_data[row_index][col]);
                                });
                            }
                        });
                    }
                });
        });
    }

    fn delete_selected_rows(&mut self, _ctx: &egui::Context) {
        let selected_count = self
            .selected_rows
            .iter()
            .filter(|&&selected| selected)
            .count();
        if selected_count > 0 {
            self.show_confirmation_dialog = true;
        }
    }

    fn perform_delete(&mut self) {
        // Create a vector of indices to remove
        let indices_to_remove: Vec<usize> = self
            .selected_rows
            .iter()
            .enumerate()
            .filter_map(|(i, &selected)| if selected { Some(i) } else { None })
            .collect();

        // Remove rows in reverse order to maintain correct indices
        for &index in indices_to_remove.iter().rev() {
            if index < self.table_data.len() {
                self.table_data.remove(index);
            }
        }

        // Update selected_rows and num_rows
        self.selected_rows = vec![false; self.table_data.len()];
        self.num_rows = self.table_data.len();
    }
}
