use egui::{Button, ScrollArea, TextEdit, Ui, Window};
use egui_extras::{Column, TableBuilder};
use tokio::runtime::Builder;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct ThisApp {
    table_data: Vec<Vec<String>>,
    num_rows: usize,
    num_columns: usize,
    selected_rows: Vec<bool>,
    show_confirmation_dialog: bool,
    add_row_window_open: bool,
    new_row_name: String,
    new_row_link: String,
    new_row_css_selector: String,
    new_row_value: String,
    is_fetching: bool,
}

impl Default for ThisApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            table_data: vec![vec![
                "Name".to_owned(),
                "Link".to_owned(),
                "CSS Selector".to_owned(),
                "Previous Value".to_owned(),
                "Latest Value".to_owned(),
            ]],
            num_rows: 0,
            num_columns: 5,
            selected_rows: vec![false, false],
            show_confirmation_dialog: false,
            add_row_window_open: false,
            new_row_name: String::new(),
            new_row_link: String::new(),
            new_row_css_selector: String::new(),
            new_row_value: String::new(),
            is_fetching: false,
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
                    self.add_row_window_open = true;
                }

                // Add Delete Selected Rows button
                if ui.button("🗑 Delete Selected Rows").clicked() {
                    self.delete_selected_rows(ctx);
                }

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

        if self.add_row_window_open {
            let mut open = self.add_row_window_open;
            Window::new("Add New Row").open(&mut open).show(ctx, |ui| {
                let this = &mut *self; // Reborrow self inside the closure
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.add(TextEdit::singleline(&mut this.new_row_name));
                });
                ui.horizontal(|ui| {
                    ui.label("Link:");
                    ui.add(TextEdit::singleline(&mut this.new_row_link));
                });
                ui.horizontal(|ui| {
                    ui.label("CSS Selector:");
                    ui.add(TextEdit::singleline(&mut this.new_row_css_selector));
                });

                let mut fetched_value = String::new();
                ui.horizontal(|ui| {
                    if ui.button("Fetch Value").clicked() && !this.is_fetching {
                        this.is_fetching = true;
                        let link = this.new_row_link.clone();
                        let css_selector = this.new_row_css_selector.clone();
                        let ctx = ctx.clone();

                        // Create a new Tokio runtime for this operation
                        let runtime = Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .expect("Failed to create Tokio runtime");

                        std::thread::spawn(move || {
                            runtime.block_on(async {
                                match crate::get_current_value(&link, &css_selector).await {
                                    Ok(Some(value)) => {
                                        // Handle successful fetch
                                        println!("Fetched value: {}", value);
                                        // fetched_value = value;
                                        ctx.request_repaint();
                                        // You'll need to update the UI with the fetched value here
                                    }
                                    Ok(None) => {
                                        // Handle case where value was not found
                                        println!("Value not found");
                                        ctx.request_repaint();
                                        // Update UI to show that value was not found
                                    }
                                    Err(e) => {
                                        // Handle error
                                        eprintln!("Error: {}", e);
                                        ctx.request_repaint();
                                        // Update UI to show error
                                    }
                                }
                            });
                        });
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Fetched Value:");
                    ui.add(TextEdit::singleline(&mut fetched_value).interactive(false));
                });

                if this.is_fetching {
                    ui.spinner();
                } else {
                    // self.new_row_value = fetched_value;
                }

                ui.horizontal(|ui| {
                    let add_button = ui.add_enabled(
                        !this.is_fetching && this.new_row_value.len() > 0,
                        Button::new("Add"),
                    );
                    if add_button.clicked() {
                        let new_row = vec![
                            this.new_row_name.clone(),
                            this.new_row_link.clone(),
                            this.new_row_css_selector.clone(),
                            this.new_row_value.clone(),
                            this.new_row_value.clone(),
                        ];
                        this.table_data.push(new_row);
                        this.reset_new_row_fields();
                        this.add_row_window_open = false;
                    }

                    if ui.button("Reset").clicked() {
                        this.reset_new_row_fields();
                        this.add_row_window_open = false;
                    }
                });
            });
            self.add_row_window_open = open;
        }
    }
}

impl ThisApp {
    fn reset_new_row_fields(&mut self) {
        self.new_row_name.clear();
        self.new_row_link.clear();
        self.new_row_css_selector.clear();
        self.new_row_value.clear();
        self.is_fetching = false;
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
