use std::{
    collections::VecDeque,
    thread::{self},
};

use egui::{Button, Color32, ScrollArea, TextEdit, Ui, Window};
use egui_extras::{Column, TableBuilder};
use poll_promise::Promise;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};
use ulid::Ulid;

const DEFAULT_CUSTOM_TIME_INTERVAL: u64 = 40;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ThisApp {
    table_data: Vec<ValueData>,
    column_names: Vec<String>,
    selected_rows: Vec<bool>,
    custom_time_interval: u64,
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
    fetch_value_promise: Option<Promise<(String, String)>>,
    show_error_message: bool,
    fetching_latest_values: bool,
    fetch_latest_values_promises: VecDeque<Promise<(String, String)>>,
    resolved_promises_count: usize,
    scheduled_job_setup: bool,
    mpsc_sender: std::sync::mpsc::Sender<VecDeque<(String, String)>>,
    mpsc_receiver: std::sync::mpsc::Receiver<VecDeque<(String, String)>>,
    show_custom_interval_dialog: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct ValueData {
    pub id: String,
    pub name: String,
    pub link: String,
    pub css_selector: String,
    pub previous_value: String,
    pub latest_value: String,
    pub last_updated: String,
}

impl Default for ThisApp {
    fn default() -> Self {
        let (mpsc_sender, mpsc_receiver) = std::sync::mpsc::channel();
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
            custom_time_interval: DEFAULT_CUSTOM_TIME_INTERVAL,
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
                fetching_latest_values: false,
                fetch_latest_values_promises: VecDeque::new(),
                resolved_promises_count: 0,
                scheduled_job_setup: false,
                mpsc_sender,
                mpsc_receiver,
                show_custom_interval_dialog: false,
            },
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
enum SelectorOption {
    Custom,
    AmazonItemPrice,
    AmazonBookPrice,
    EbayPrice,
}

impl SelectorOption {
    fn as_str(&self) -> &'static str {
        match self {
            SelectorOption::Custom => "Custom",
            SelectorOption::AmazonItemPrice => "Amazon Item Price",
            SelectorOption::AmazonBookPrice => "Amazon Book Price",
            SelectorOption::EbayPrice => "Ebay Price",
        }
    }

    fn get_options() -> [SelectorOption; 4] {
        [
            SelectorOption::Custom,
            SelectorOption::AmazonItemPrice,
            SelectorOption::AmazonBookPrice,
            SelectorOption::EbayPrice,
        ]
    }

    fn get_selector(&self) -> &'static str {
        match self {
            SelectorOption::Custom => "",
            SelectorOption::AmazonItemPrice => ".a-offscreen",
            SelectorOption::AmazonBookPrice => ".aok-offscreen",
            SelectorOption::EbayPrice => "div.x-price-primary span.ux-textspans",
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

        // Request a auto-repaint after 1 second
        // ctx.request_repaint_after(Duration::from_secs(1));

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            Self::menu_bar(self, ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            Self::table_ui(self, ui);
            Self::code_link(ui);
            Self::condional_components(self, ctx);
        });

        // Poll the promise in the update loop
        if self.runtime_state.show_add_row_dialog {
            if let Some(promise) = self.runtime_state.fetch_value_promise.as_mut() {
                if let Some(value) = promise.ready() {
                    self.runtime_state.show_spinner = false;
                    self.runtime_state.new_row_value = value.1.clone();
                    self.runtime_state.fetch_value_promise = None; // Clear the promise after completion
                } else {
                    self.runtime_state.show_spinner = true;
                }
            }
        }

        if self.runtime_state.fetching_latest_values {
            if let Some(promise) = self.runtime_state.fetch_latest_values_promises.front() {
                if let Some((id, value)) = promise.ready() {
                    println!("promise ready");
                    self.update_value(id.clone(), value.clone());
                    self.runtime_state.fetch_latest_values_promises.pop_front();
                    self.runtime_state.resolved_promises_count += 1;
                }
                if self.runtime_state.resolved_promises_count == self.table_data.len() {
                    self.runtime_state.fetching_latest_values = false;
                    self.runtime_state.resolved_promises_count = 0;
                }
            }
        }

        //initialize the sheduled job
        if !self.runtime_state.scheduled_job_setup {
            // self.fetch_latest_values(); //refresh the values everytime app starts
            self.sheduled_job(ctx);
            self.runtime_state.scheduled_job_setup = true;
        }

        //poll the mpsc_receiver
        if let Ok(new_values) = self.runtime_state.mpsc_receiver.try_recv() {
            for (id, value) in new_values {
                self.update_value(id, value);
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
                .column(Column::auto()) // Checkbox column
                .column(Column::auto().clip(true))
                .column(Column::auto().clip(true))
                .columns(Column::auto().clip(true), self.column_names.len() - 2)
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
                        body.row(25.0, |mut row| {
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
            // Add new rows
            if ui.button("➕ Add Row").clicked() {
                self.runtime_state.show_add_row_dialog = true;
            }

            // Delete Selected Rows button
            if ui.button("🗑 Delete Selected Rows").clicked() {
                Self::delete_selected_rows(self);
            }

            // Fetch latest values button
            if ui
                .add_enabled(
                    !self.runtime_state.fetching_latest_values,
                    egui::Button::new("🔄 Fetch Latest Values"),
                )
                .clicked()
            {
                self.fetch_latest_values();
            }

            ui.menu_button("📐 Settings", |ui| self.nested_menus(ui));

            // dark/light mode toggle button
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::widgets::global_theme_preference_buttons(ui);
            });
        });
    }

    fn nested_menus(&mut self, ui: &mut egui::Ui) {
        ui.set_max_width(200.0); // To make sure we wrap long text

        if ui.button("🔔 Test notification").clicked() {
            crate::show_notifcation(&"NAME".to_string(), &"X".to_string(), &"Y".to_string());
            ui.close_menu();
        }

        if ui.button("⏳ Set custom time interval").clicked() {
            self.runtime_state.show_custom_interval_dialog = true;
            ui.close_menu();
        }
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
        Self::add_row_dialog(self, ctx);
        Self::delete_confirmation_dialog(self, ctx);
        Self::custom_interval_dialog(self, ctx);
    }

    fn add_row_dialog(&mut self, ctx: &egui::Context) {
        if self.runtime_state.show_add_row_dialog {
            let mut open = self.runtime_state.show_add_row_dialog;
            Window::new("Add New Row")
                .open(&mut open)
                .collapsible(false)
                .auto_sized()
                .show(ctx, |ui| {
                    let this = &mut *self; // Reborrow self inside the closure
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add(TextEdit::singleline(&mut this.runtime_state.new_row_name).hint_text("enter any name"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Link:");
                        ui.add(TextEdit::singleline(&mut this.runtime_state.new_row_link).hint_text("enter link of the webpage"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("CSS Selector:");
                        let options = SelectorOption::get_options();
                        let current_option = options.iter()
                        .find(|&&opt| opt.get_selector() == this.runtime_state.new_row_css_selector)
                        .copied()
                        .unwrap_or(SelectorOption::Custom);

                        let mut selected_option = current_option;
                        egui::ComboBox::from_label("")
                            .selected_text(selected_option.as_str())
                            .show_ui(ui, |ui| {
                                for option in options {
                                    if ui.selectable_value(&mut selected_option, option, option.as_str()).clicked() {
                                        this.runtime_state.new_row_css_selector = selected_option.get_selector().to_string();
                                    }
                                }
                            });

                        ui.add_enabled(
                            selected_option == SelectorOption::Custom,
                            TextEdit::singleline(&mut this.runtime_state.new_row_css_selector)
                                .hint_text("enter css selector of the thing you want to track"),
                        );
                        ui.hyperlink_to("what?", "https://github.com/Ashu999/web-value-tracker?tab=readme-ov-file#css-selectors-how");
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Fetch Value").clicked() {
                            let link = this.runtime_state.new_row_link.clone();
                            let css_selector = this.runtime_state.new_row_css_selector.clone();
                            this.runtime_state.show_spinner = true;

                            this.runtime_state.fetch_value_promise = Some(
                                crate::get_web_value_promise(String::new(), link, css_selector),
                            );
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
                                .interactive(false).hint_text("will autofill after fetching value"),
                        );
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Reset").clicked() {
                            this.reset_new_row_fields();
                        }

                        let add_button = ui.add_enabled(
                            !this.runtime_state.new_row_value.is_empty(),
                            Button::new("Add"),
                        );
                        if add_button.clicked() {
                            this.add_new_row();
                            this.selected_rows.push(false);
                            this.reset_new_row_fields();
                        }
                    });
                });
            self.runtime_state.show_add_row_dialog = open;
        }
    }

    fn add_new_row(&mut self) {
        let cur_date_time = crate::get_current_date_time();
        let new_row = ValueData {
            id: Ulid::new().to_string(),
            name: self.runtime_state.new_row_name.clone(),
            link: self.runtime_state.new_row_link.clone(),
            css_selector: self.runtime_state.new_row_css_selector.clone(),
            previous_value: self.runtime_state.new_row_value.clone(),
            latest_value: self.runtime_state.new_row_value.clone(),
            last_updated: cur_date_time,
        };
        self.table_data.push(new_row);
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
                            Self::perform_delete(self);
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

    fn custom_interval_dialog(&mut self, ctx: &egui::Context) {
        if self.runtime_state.show_custom_interval_dialog {
            let mut open = self.runtime_state.show_custom_interval_dialog;
            Window::new("Set Custom Time Interval")
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Values will be checked every: {} minutes",
                        self.custom_time_interval
                    ));

                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut self.custom_time_interval)
                                .range(1..=u64::MAX),
                        );
                        ui.label("minutes");
                    });
                    ui.colored_label(Color32::RED, "* Restart the app to apply changes.");
                });
            self.runtime_state.show_custom_interval_dialog = open;
        }
    }

    fn update_value(&mut self, id: String, value: String) {
        println!("Updating value for ID: {}, Value: {}", id, value);
        if let Some(index) = self.table_data.iter().position(|row| row.id == id) {
            let row = &mut self.table_data[index];
            row.previous_value = row.latest_value.clone();
            row.latest_value = value;
            row.last_updated = crate::get_current_date_time();
        }
    }

    fn fetch_latest_values(&mut self) {
        self.runtime_state.fetching_latest_values = true;
        self.runtime_state.fetch_latest_values_promises =
            crate::fetch_latest_values_promises(&self.table_data);
    }

    fn sheduled_job(&mut self, ctx: &egui::Context) -> thread::JoinHandle<()> {
        println!("sheduled_job called");
        let ctx = ctx.clone();
        let mut table_data = self.table_data.clone();
        let sender = self.runtime_state.mpsc_sender.clone();
        let custom_time_interval = self.custom_time_interval;
        println!("custom_time_interval: {}", custom_time_interval);

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let jobs_scheduler = JobScheduler::new().await.unwrap();
                jobs_scheduler
                    .add(
                        Job::new_repeated(
                            Duration::from_secs(custom_time_interval * 60),
                            move |_uuid, _l| {
                                // flag.store(true, Ordering::SeqCst);
                                println!(
                                    "sheduled_job: flag set at {}",
                                    crate::get_current_date_time()
                                );
                                let new_values =
                                    crate::fetch_latest_values_and_notify_blocking(&mut table_data);
                                if let Err(e) = sender.send(new_values) {
                                    eprintln!("Failed to send new_values: {:?}", e);
                                }
                                ctx.request_repaint();
                            },
                        )
                        .unwrap(),
                    )
                    .await
                    .unwrap();
                jobs_scheduler.start().await.unwrap();
                tokio::signal::ctrl_c().await.unwrap();
            });
        })
    }
}
