use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Color32, Layout, RichText, Stroke};

use crate::engine::MouseButton;
use crate::platform::windows::{LiveClickerConfig, PrecisionClicker};

use super::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Dashboard,
    AutoClicker,
    Macros,
    KeyRemap,
    Profiles,
    Settings,
}

#[derive(Debug, Clone)]
struct MacroRow {
    name: String,
    trigger: String,
    action: String,
    enabled: bool,
}

#[derive(Debug, Clone)]
struct RemapRow {
    source: String,
    target: String,
    enabled: bool,
}

pub struct VxClickApp {
    page: Page,
    cps: f64,
    button: MouseButton,
    coarse_wait_threshold_us: f64,
    spin_window_us: f64,
    running: Arc<AtomicBool>,
    stop_requested: Arc<AtomicBool>,
    click_count: Arc<AtomicU64>,
    last_error: Arc<Mutex<Option<String>>>,
    worker: Option<JoinHandle<()>>,
    measured_cps: f64,
    sample_clicks: u64,
    sample_time: Instant,
    session_started: Option<Instant>,
    macros: Vec<MacroRow>,
    mappings: Vec<RemapRow>,
    profiles: Vec<String>,
    active_profile: usize,
    start_with_windows: bool,
    minimize_to_tray: bool,
    high_performance: bool,
}

impl Default for VxClickApp {
    fn default() -> Self {
        Self {
            page: Page::Dashboard,
            cps: 250.0,
            button: MouseButton::Left,
            coarse_wait_threshold_us: 2_000.0,
            spin_window_us: 350.0,
            running: Arc::new(AtomicBool::new(false)),
            stop_requested: Arc::new(AtomicBool::new(false)),
            click_count: Arc::new(AtomicU64::new(0)),
            last_error: Arc::new(Mutex::new(None)),
            worker: None,
            measured_cps: 0.0,
            sample_clicks: 0,
            sample_time: Instant::now(),
            session_started: None,
            macros: vec![MacroRow {
                name: "Quick Action".into(),
                trigger: "Mouse 4".into(),
                action: "F".into(),
                enabled: true,
            }],
            mappings: vec![RemapRow {
                source: "Mouse 5".into(),
                target: "Ctrl + C".into(),
                enabled: true,
            }],
            profiles: vec!["Default".into()],
            active_profile: 0,
            start_with_windows: false,
            minimize_to_tray: true,
            high_performance: true,
        }
    }
}

impl VxClickApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        Self::default()
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    fn start_clicking(&mut self) {
        self.reap_worker();
        if self.is_running() {
            return;
        }

        self.stop_requested.store(false, Ordering::Release);
        self.running.store(true, Ordering::Release);
        self.session_started = Some(Instant::now());
        self.sample_clicks = self.click_count.load(Ordering::Relaxed);
        self.sample_time = Instant::now();
        self.measured_cps = 0.0;
        if let Ok(mut error) = self.last_error.lock() {
            *error = None;
        }

        let stop = Arc::clone(&self.stop_requested);
        let running = Arc::clone(&self.running);
        let clicks = Arc::clone(&self.click_count);
        let last_error = Arc::clone(&self.last_error);
        let config = LiveClickerConfig {
            cps: self.cps,
            button: self.button,
            coarse_wait_threshold_us: self.coarse_wait_threshold_us,
            spin_window_us: self.spin_window_us,
        };

        self.worker = Some(std::thread::spawn(move || {
            let result = PrecisionClicker::new()
                .and_then(|engine| engine.run_until_stopped(config, &stop, &clicks));
            if let Err(message) = result {
                if let Ok(mut error) = last_error.lock() {
                    *error = Some(message);
                }
            }
            running.store(false, Ordering::Release);
        }));
    }

    fn stop_clicking(&mut self) {
        self.stop_requested.store(true, Ordering::Release);
    }

    fn reap_worker(&mut self) {
        if !self.is_running() && self.worker.is_some() {
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }

    fn refresh_metrics(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.sample_time).as_secs_f64();
        if elapsed >= 0.35 {
            let clicks = self.click_count.load(Ordering::Relaxed);
            self.measured_cps = if self.is_running() {
                (clicks.saturating_sub(self.sample_clicks)) as f64 / elapsed
            } else {
                0.0
            };
            self.sample_clicks = clicks;
            self.sample_time = now;
        }
        self.reap_worker();
    }

    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("vx_sidebar")
            .exact_width(205.0)
            .resizable(false)
            .frame(egui::Frame::default().fill(theme::SIDEBAR))
            .show(ctx, |ui| {
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.label(
                        RichText::new("VxClick")
                            .size(24.0)
                            .strong()
                            .color(theme::ACCENT_BRIGHT),
                    );
                });
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.label(RichText::new("WINDOWS AUTOMATION").size(9.0).color(theme::MUTED));
                });
                ui.add_space(24.0);

                self.nav_button(ui, Page::Dashboard, "Dashboard");
                self.nav_button(ui, Page::AutoClicker, "Auto Clicker");
                self.nav_button(ui, Page::Macros, "Macros");
                self.nav_button(ui, Page::KeyRemap, "Key Remap");
                self.nav_button(ui, Page::Profiles, "Profiles");
                self.nav_button(ui, Page::Settings, "Settings");

                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    ui.add_space(18.0);
                    ui.horizontal(|ui| {
                        ui.add_space(14.0);
                        let (dot, text) = if self.is_running() {
                            (theme::GREEN, "Running")
                        } else {
                            (theme::ACCENT, "Ready")
                        };
                        ui.colored_label(dot, "●");
                        ui.label(RichText::new(text).color(theme::MUTED));
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(14.0);
                        ui.label(RichText::new("v0.1.0").size(10.0).color(theme::MUTED));
                    });
                });
            });
    }

    fn nav_button(&mut self, ui: &mut egui::Ui, page: Page, label: &str) {
        let selected = self.page == page;
        let fill = if selected {
            Color32::from_rgb(7, 65, 118)
        } else {
            Color32::TRANSPARENT
        };
        let stroke = if selected {
            Stroke::new(1.0, theme::ACCENT)
        } else {
            Stroke::NONE
        };
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            let button = egui::Button::new(
                RichText::new(label)
                    .size(14.0)
                    .color(if selected { Color32::WHITE } else { theme::MUTED }),
            )
            .fill(fill)
            .stroke(stroke);
            if ui.add_sized([180.0, 38.0], button).clicked() {
                self.page = page;
            }
        });
    }

    fn top_header(&self, ui: &mut egui::Ui, title: &str, subtitle: &str) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(title).size(28.0).strong());
                ui.label(RichText::new(subtitle).size(13.0).color(theme::MUTED));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let running = self.is_running();
                let color = if running { theme::GREEN } else { theme::ACCENT };
                let label = if running { "●  Running" } else { "●  Ready" };
                ui.label(RichText::new(label).color(color).strong());
            });
        });
        ui.add_space(18.0);
    }

    fn dashboard(&mut self, ui: &mut egui::Ui) {
        self.top_header(ui, "Dashboard", "Automate more. Do more.");

        let clicks = self.click_count.load(Ordering::Relaxed);
        let uptime = self
            .session_started
            .map(|started| format_duration(started.elapsed()))
            .unwrap_or_else(|| "00:00:00".into());
        let status = if self.is_running() { "Running" } else { "Ready" };

        ui.columns(4, |columns| {
            metric_card(&mut columns[0], "Clicks", &format_number(clicks), theme::ACCENT_BRIGHT);
            metric_card(
                &mut columns[1],
                "CPS (actual)",
                &format!("{:.1}", self.measured_cps),
                theme::ACCENT,
            );
            metric_card(&mut columns[2], "Uptime", &uptime, theme::PURPLE);
            metric_card(
                &mut columns[3],
                "Status",
                status,
                if self.is_running() { theme::GREEN } else { theme::ACCENT },
            );
        });

        ui.add_space(14.0);
        ui.columns(2, |columns| {
            section_card(&mut columns[0], |ui| {
                ui.label(RichText::new("Quick Start").size(17.0).strong());
                ui.label(RichText::new("Start the precision click engine immediately.").color(theme::MUTED));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.is_running(),
                            egui::Button::new(RichText::new("Start Clicking").strong())
                                .fill(Color32::from_rgb(0, 111, 219)),
                        )
                        .clicked()
                    {
                        self.start_clicking();
                    }
                    if ui
                        .add_enabled(
                            self.is_running(),
                            egui::Button::new("Stop").fill(Color32::from_rgb(97, 32, 49)),
                        )
                        .clicked()
                    {
                        self.stop_clicking();
                    }
                    if ui.button("Configure").clicked() {
                        self.page = Page::AutoClicker;
                    }
                });
            });

            section_card(&mut columns[1], |ui| {
                ui.label(RichText::new("Active Profile").size(17.0).strong());
                ui.label(RichText::new("Settings are isolated per profile.").color(theme::MUTED));
                ui.add_space(10.0);
                let profile = self
                    .profiles
                    .get(self.active_profile)
                    .map(String::as_str)
                    .unwrap_or("Default");
                ui.horizontal(|ui| {
                    ui.label(RichText::new(profile).size(18.0).color(theme::ACCENT_BRIGHT));
                    if ui.button("Manage").clicked() {
                        self.page = Page::Profiles;
                    }
                });
            });
        });

        ui.add_space(14.0);
        section_card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Precision Mode Enabled")
                            .size(18.0)
                            .strong()
                            .color(theme::ACCENT_BRIGHT),
                    );
                    ui.label(
                        RichText::new(format!(
                            "Target {:.1} CPS · {:.3} ms interval · QPC absolute deadlines",
                            self.cps,
                            1_000.0 / self.cps.max(0.001)
                        ))
                        .color(theme::MUTED),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("HIGH PERFORMANCE").size(11.0).color(theme::ACCENT));
                });
            });
        });

        if let Some(message) = self.last_error.lock().ok().and_then(|error| error.clone()) {
            ui.add_space(12.0);
            ui.colored_label(theme::RED, format!("Engine error: {message}"));
        }
    }

    fn auto_clicker(&mut self, ui: &mut egui::Ui) {
        self.top_header(ui, "Auto Clicker", "High precision click scheduling for Windows.");

        section_card(ui, |ui| {
            ui.label(RichText::new("Click Rate").size(17.0).strong());
            ui.label(RichText::new("1–20,000 CPS. Sub-millisecond intervals use the precision path.").color(theme::MUTED));
            ui.add_space(8.0);
            ui.add(
                egui::Slider::new(&mut self.cps, 1.0..=20_000.0)
                    .logarithmic(true)
                    .text("CPS"),
            );
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Interval: {:.3} µs", 1_000_000.0 / self.cps)).color(theme::ACCENT_BRIGHT));
                ui.separator();
                ui.label(RichText::new(format!("Target: {:.1} CPS", self.cps)).color(theme::MUTED));
            });
        });

        ui.add_space(14.0);
        ui.columns(2, |columns| {
            section_card(&mut columns[0], |ui| {
                ui.label(RichText::new("Mouse Button").size(17.0).strong());
                ui.add_space(8.0);
                egui::ComboBox::from_id_salt("mouse_button")
                    .selected_text(mouse_button_name(self.button))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.button, MouseButton::Left, "Left");
                        ui.selectable_value(&mut self.button, MouseButton::Right, "Right");
                        ui.selectable_value(&mut self.button, MouseButton::Middle, "Middle");
                        ui.selectable_value(&mut self.button, MouseButton::X1, "Mouse 4 / X1");
                        ui.selectable_value(&mut self.button, MouseButton::X2, "Mouse 5 / X2");
                    });
                ui.label(RichText::new("Single click output through Windows SendInput.").color(theme::MUTED));
            });

            section_card(&mut columns[1], |ui| {
                ui.label(RichText::new("Engine").size(17.0).strong());
                ui.add_space(8.0);
                ui.label(RichText::new("QPC absolute deadlines").color(theme::ACCENT_BRIGHT));
                ui.label(RichText::new("Hybrid sleep / yield / spin").color(theme::MUTED));
                ui.label(RichText::new("No accumulated sleep drift").color(theme::MUTED));
                ui.label(RichText::new("Long stalls re-anchor instead of burst-catching up").color(theme::MUTED));
            });
        });

        ui.add_space(14.0);
        section_card(ui, |ui| {
            ui.horizontal(|ui| {
                let running = self.is_running();
                if ui
                    .add_enabled(
                        !running,
                        egui::Button::new(RichText::new("Start").size(15.0).strong())
                            .fill(Color32::from_rgb(0, 111, 219)),
                    )
                    .clicked()
                {
                    self.start_clicking();
                }
                if ui
                    .add_enabled(
                        running,
                        egui::Button::new(RichText::new("Stop").size(15.0).strong())
                            .fill(Color32::from_rgb(97, 32, 49)),
                    )
                    .clicked()
                {
                    self.stop_clicking();
                }
                ui.separator();
                ui.label(
                    RichText::new(format!("Actual {:.1} CPS", self.measured_cps))
                        .color(theme::ACCENT_BRIGHT),
                );
                ui.label(
                    RichText::new(format!(
                        "{} clicks",
                        format_number(self.click_count.load(Ordering::Relaxed))
                    ))
                    .color(theme::MUTED),
                );
            });
        });
    }

    fn macros(&mut self, ui: &mut egui::Ui) {
        self.top_header(ui, "Macros", "Build keyboard and mouse action sequences.");
        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::new("+ New Macro").fill(Color32::from_rgb(0, 91, 178)))
                .clicked()
            {
                let number = self.macros.len() + 1;
                self.macros.push(MacroRow {
                    name: format!("Macro {number}"),
                    trigger: "F6".into(),
                    action: "Left Click".into(),
                    enabled: true,
                });
            }
            ui.label(RichText::new("Recorder + timed sequence executor is the next engine block.").color(theme::MUTED));
        });
        ui.add_space(12.0);

        for (index, row) in self.macros.iter_mut().enumerate() {
            section_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut row.enabled, "");
                    ui.add_sized([180.0, 30.0], egui::TextEdit::singleline(&mut row.name));
                    ui.label(RichText::new("Trigger").color(theme::MUTED));
                    ui.add_sized([110.0, 30.0], egui::TextEdit::singleline(&mut row.trigger));
                    ui.label(RichText::new("Action").color(theme::MUTED));
                    ui.add_sized([180.0, 30.0], egui::TextEdit::singleline(&mut row.action));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format!("#{:02}", index + 1)).color(theme::MUTED));
                    });
                });
            });
            ui.add_space(8.0);
        }
    }

    fn key_remap(&mut self, ui: &mut egui::Ui) {
        self.top_header(ui, "Key Remap", "Map any keyboard or mouse trigger to another action.");
        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::new("+ Add Mapping").fill(Color32::from_rgb(0, 91, 178)))
                .clicked()
            {
                self.mappings.push(RemapRow {
                    source: "Caps Lock".into(),
                    target: "F".into(),
                    enabled: true,
                });
            }
            ui.label(
                RichText::new("Low-level keyboard/mouse hooks will power global remapping.")
                    .color(theme::MUTED),
            );
        });
        ui.add_space(12.0);

        for row in &mut self.mappings {
            section_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut row.enabled, "");
                    ui.label(RichText::new("FROM").size(10.0).color(theme::MUTED));
                    ui.add_sized([180.0, 30.0], egui::TextEdit::singleline(&mut row.source));
                    ui.label(RichText::new("→").size(20.0).color(theme::ACCENT_BRIGHT));
                    ui.label(RichText::new("TO").size(10.0).color(theme::MUTED));
                    ui.add_sized([220.0, 30.0], egui::TextEdit::singleline(&mut row.target));
                });
            });
            ui.add_space(8.0);
        }
    }

    fn profiles(&mut self, ui: &mut egui::Ui) {
        self.top_header(ui, "Profiles", "Keep click, macro and remap settings separated.");
        if ui
            .add(egui::Button::new("+ New Profile").fill(Color32::from_rgb(0, 91, 178)))
            .clicked()
        {
            self.profiles.push(format!("Profile {}", self.profiles.len() + 1));
        }
        ui.add_space(12.0);

        for (index, profile) in self.profiles.iter().enumerate() {
            let active = self.active_profile == index;
            let response = egui::Frame::group(ui.style())
                .fill(if active { Color32::from_rgb(8, 49, 86) } else { theme::CARD })
                .stroke(Stroke::new(1.0, if active { theme::ACCENT } else { theme::BORDER }))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(profile).size(17.0).strong());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if active {
                                ui.label(RichText::new("ACTIVE").size(10.0).color(theme::GREEN));
                            } else if ui.button("Activate").clicked() {
                                self.active_profile = index;
                            }
                        });
                    });
                });
            let _ = response;
            ui.add_space(8.0);
        }
    }

    fn settings(&mut self, ui: &mut egui::Ui) {
        self.top_header(ui, "Settings", "Performance, startup and precision controls.");

        section_card(ui, |ui| {
            ui.label(RichText::new("Windows").size(17.0).strong());
            ui.checkbox(&mut self.start_with_windows, "Start VxClick with Windows");
            ui.checkbox(&mut self.minimize_to_tray, "Minimize to system tray");
            ui.checkbox(&mut self.high_performance, "Use high-performance worker priority");
        });

        ui.add_space(14.0);
        section_card(ui, |ui| {
            ui.label(RichText::new("Precision Tuning").size(17.0).strong());
            ui.label(RichText::new("Advanced values. Defaults prioritize low jitter without spinning for the full interval.").color(theme::MUTED));
            ui.add_space(8.0);
            ui.add(
                egui::Slider::new(&mut self.spin_window_us, 50.0..=800.0)
                    .text("Spin window (µs)"),
            );
            ui.add(
                egui::Slider::new(&mut self.coarse_wait_threshold_us, 500.0..=5_000.0)
                    .text("Coarse wait threshold (µs)"),
            );
        });

        ui.add_space(14.0);
        section_card(ui, |ui| {
            ui.label(RichText::new("Theme").size(17.0).strong());
            ui.label(RichText::new("VxClick Dark · Navy / Electric Cyan").color(theme::ACCENT_BRIGHT));
            ui.label(RichText::new("Built around the supplied Windows app icon.").color(theme::MUTED));
        });
    }
}

impl eframe::App for VxClickApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.refresh_metrics();
        self.sidebar(ctx);

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(theme::BG))
            .show(ctx, |ui| {
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.add_space(18.0);
                    ui.vertical(|ui| {
                        ui.set_max_width((ui.available_width() - 18.0).max(200.0));
                        match self.page {
                            Page::Dashboard => self.dashboard(ui),
                            Page::AutoClicker => self.auto_clicker(ui),
                            Page::Macros => self.macros(ui),
                            Page::KeyRemap => self.key_remap(ui),
                            Page::Profiles => self.profiles(ui),
                            Page::Settings => self.settings(ui),
                        }
                    });
                });
            });

        if self.is_running() {
            ctx.request_repaint_after(Duration::from_millis(75));
        } else {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }
}

impl Drop for VxClickApp {
    fn drop(&mut self) {
        self.stop_requested.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn section_card<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::group(ui.style())
        .fill(theme::CARD)
        .stroke(Stroke::new(1.0, theme::BORDER.gamma_multiply(0.65)))
        .show(ui, |ui| {
            ui.add_space(4.0);
            let result = add_contents(ui);
            ui.add_space(4.0);
            result
        })
        .inner
}

fn metric_card(ui: &mut egui::Ui, label: &str, value: &str, accent: Color32) {
    section_card(ui, |ui| {
        ui.set_min_height(74.0);
        ui.label(RichText::new(label).size(12.0).color(theme::MUTED));
        ui.add_space(4.0);
        ui.label(RichText::new(value).size(23.0).strong().color(accent));
    });
}

fn mouse_button_name(button: MouseButton) -> &'static str {
    match button {
        MouseButton::Left => "Left",
        MouseButton::Right => "Right",
        MouseButton::Middle => "Middle",
        MouseButton::X1 => "Mouse 4 / X1",
        MouseButton::X2 => "Mouse 5 / X2",
    }
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let hours = seconds / 3_600;
    let minutes = (seconds % 3_600) / 60;
    let seconds = seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

fn format_number(value: u64) -> String {
    let raw = value.to_string();
    let mut output = String::with_capacity(raw.len() + raw.len() / 3);
    for (index, ch) in raw.chars().enumerate() {
        if index > 0 && (raw.len() - index) % 3 == 0 {
            output.push(',');
        }
        output.push(ch);
    }
    output
}
