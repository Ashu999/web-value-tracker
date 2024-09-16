use egui::{Button, ScrollArea, TextEdit, Ui, Window};
use egui_extras::{Column, TableBuilder};
use poll_promise::Promise;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ThisApp {
    table_data: Vec<ValueData>,
    column_names: Vec<String>,
    selected_rows: Vec<bool>,
    #[serde(skip)]
    runtime_state: RuntimeState,
}

struct RuntimeState {
    show_delete_confirmation_dialog: bool,
    show_add_row_dialog: bool,
    new_row_name: String,
    new_row_link: String,
    new_row_css_selector: String,
    show_spinner: bool,
    new_row_value: String,
    fetch_value_promise: Option<Promise<String>>,
    show_error_message: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
struct ValueData {
    name: String,
    link: String,
    css_selector: String,
    previous_value: String,
    latest_value: String,
    last_updated: String,
}

impl Default for ThisApp {
    fn default() -> Self {
        Self {
            table_data: Vec::new(),
            column_names: vec![
                "Name".to_owned(),
                "Link".to_owned(),
                "CSS Selector".to_owned(),
                "Previous Value".to_owned(),
                "Latest Value".to_owned(),
                "Last Updated".to_owned(),
            ],
            selected_rows: vec![false; 0],
            runtime_state: RuntimeState {
                show_delete_confirmation_dialog: false,
                show_add_row_dialog: false,
                new_row_name: String::new(),
                new_row_link: String::new(),
                new_row_css_selector: String::new(),
                show_spinner: false,
                new_row_value: String::new(),
                fetch_value_promise: None,
                show_error_message: false,
            },
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
            ThisApp::menu_bar(self, ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ThisApp::table_ui(self, ui);
            ThisApp::code_link(ui);
            ThisApp::condional_components(self, ctx);
        });

        // Poll the promise in the update loop
        if let Some(promise) = self.runtime_state.fetch_value_promise.as_mut() {
            if let Some(value) = promise.ready() {
                self.runtime_state.show_spinner = false;
                self.runtime_state.new_row_value = value.clone();
                self.runtime_state.fetch_value_promise = None; // Clear the promise after completion
            } else {
                self.runtime_state.show_spinner = true;
            }
        }
    }
}

impl ThisApp {
    fn reset_new_row_fields(&mut self) {
        self.runtime_state.new_row_name.clear();
        self.runtime_state.new_row_link.clear();
        self.runtime_state.new_row_css_selector.clear();
        self.runtime_state.new_row_value.clear();
        self.runtime_state.show_spinner = false;
        self.runtime_state.show_error_message = false;
    }

    fn table_ui(&mut self, ui: &mut Ui) {
        ScrollArea::horizontal().show(ui, |ui| {
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .columns(Column::auto(), 1) // Checkbox column
                .columns(Column::auto(), self.column_names.len())
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("Select");
                    });
                    for col_name in &self.column_names {
                        header.col(|ui| {
                            ui.strong(col_name);
                        });
                    }
                })
                .body(|mut body| {
                    for (row_index, row_data) in self.table_data.iter().enumerate() {
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
                            let row_values = vec![
                                &row_data.name,
                                &row_data.link,
                                &row_data.css_selector,
                                &row_data.previous_value,
                                &row_data.latest_value,
                                &row_data.last_updated,
                            ];
                            for value in row_values {
                                row.col(|ui| {
                                    let text_color = if row_is_selected {
                                        ui.ctx().style().visuals.strong_text_color()
                                    } else {
                                        ui.ctx().style().visuals.text_color()
                                    };
                                    ui.colored_label(text_color, value);
                                });
                            }
                        });
                    }
                });
        });
    }

    fn code_link(ui: &mut Ui) {
        ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
            ui.add(egui::github_link_file!(
                "https://github.com/Ashu999/web-value-tracker/blob/main/",
                "Source code."
            ));
            egui::warn_if_debug_build(ui);
        });
    }

    fn menu_bar(&mut self, ui: &mut Ui) {
        egui::menu::bar(ui, |ui| {
            // Add button to add new rows
            if ui.button("➕ Add Row").clicked() {
                self.runtime_state.show_add_row_dialog = true;
            }

            // Add Delete Selected Rows button
            if ui.button("🗑 Delete Selected Rows").clicked() {
                ThisApp::delete_selected_rows(self);
            }

            // dark/light mode toggle button
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });
    }

    fn delete_selected_rows(&mut self) {
        let selected_count = self
            .selected_rows
            .iter()
            .filter(|&&selected| selected)
            .count();
        if selected_count > 0 {
            self.runtime_state.show_delete_confirmation_dialog = true;
        }
    }

    fn condional_components(&mut self, ctx: &egui::Context) {
        ThisApp::add_row_dialog(self, ctx);
        ThisApp::delete_confirmation_dialog(self, ctx);
    }

    fn add_row_dialog(&mut self, ctx: &egui::Context) {
        if self.runtime_state.show_add_row_dialog {
            let mut open = self.runtime_state.show_add_row_dialog;
            Window::new("Add New Row")
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    let this = &mut *self; // Reborrow self inside the closure
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add(TextEdit::singleline(&mut this.runtime_state.new_row_name));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Link:");
                        ui.add(TextEdit::singleline(&mut this.runtime_state.new_row_link));
                    });
                    ui.horizontal(|ui| {
                        ui.label("CSS Selector:");
                        ui.add(TextEdit::singleline(
                            &mut this.runtime_state.new_row_css_selector,
                        ));
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Fetch Value").clicked() {
                            let link = this.runtime_state.new_row_link.clone();
                            let css_selector = this.runtime_state.new_row_css_selector.clone();
                            this.runtime_state.show_spinner = true;

                            this.runtime_state.fetch_value_promise =
                                Some(crate::get_web_value(link, css_selector));
                        }
                        if this.runtime_state.show_spinner {
                            ui.spinner();
                        }
                        if this.runtime_state.show_error_message {
                            ui.horizontal(|ui| {
                                ui.label("Error fetching value: check Link or CSS Selector.");
                            });
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Fetched value:");
                        ui.add(
                            TextEdit::singleline(&mut this.runtime_state.new_row_value)
                                .interactive(false),
                        );
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Reset").clicked() {
                            this.reset_new_row_fields();
                        }

                        let add_button = ui.add_enabled(
                            this.runtime_state.new_row_value.len() > 0,
                            Button::new("Add"),
                        );
                        if add_button.clicked() {
                            let cur_date_time =
                                chrono::Local::now().format("%b %d %H:%M:%S %Y").to_string();
                            let new_row = ValueData {
                                name: this.runtime_state.new_row_name.clone(),
                                link: this.runtime_state.new_row_link.clone(),
                                css_selector: this.runtime_state.new_row_css_selector.clone(),
                                previous_value: this.runtime_state.new_row_value.clone(),
                                latest_value: this.runtime_state.new_row_value.clone(),
                                last_updated: cur_date_time,
                            };
                            this.table_data.push(new_row);
                            this.selected_rows.push(false);
                            this.reset_new_row_fields();
                        }
                    });
                });
            self.runtime_state.show_add_row_dialog = open;
        }
    }

    fn delete_confirmation_dialog(&mut self, ctx: &egui::Context) {
        if self.runtime_state.show_delete_confirmation_dialog {
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
                            ThisApp::perform_delete(self);
                            self.runtime_state.show_delete_confirmation_dialog = false;
                        }
                        if ui.button("No").clicked() {
                            self.runtime_state.show_delete_confirmation_dialog = false;
                        }
                    });
                });
        }
    }

    fn perform_delete(&mut self) {
        let indices_to_remove: Vec<usize> = self
            .selected_rows
            .iter()
            .enumerate()
            .filter_map(|(i, &selected)| if selected { Some(i) } else { None })
            .collect();

        for &index in indices_to_remove.iter().rev() {
            if index < self.table_data.len() {
                self.table_data.remove(index);
                self.selected_rows.remove(index);
            }
        }
    }
}
