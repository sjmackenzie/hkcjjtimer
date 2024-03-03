use web_time::{Duration, Instant};

const MT: u64 = 10;
const PFT: u64 = MT / 2;
const HPFT: u64 = PFT / 2;

#[derive(PartialEq)]
enum MatchStage {
    FirstHalfPenaltyFree,
    SecondHalfPenaltyFree,
    Penalty,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct CjjTimer {
    #[serde(skip)]
    start_non_engaged_instant: Instant,
    #[serde(skip)]
    total_non_engaged_duration: Duration,
    #[serde(skip)]
    start_regulation_instant: Instant,
    #[serde(skip)]
    total_regulation_duration: Duration,
    #[serde(skip)]
    regulation_input: u64,
    #[serde(skip)]
    regulation_duration: Duration,
    #[serde(skip)]
    penalty_free_input: u64,
    #[serde(skip)]
    penalty_free_duration: Duration,
    #[serde(skip)]
    half_penalty_free_duration: Duration,
    #[serde(skip)]
    total_penalty_duration: Duration,
    #[serde(skip)]
    over_time: bool,
    #[serde(skip)]
    state: RegulationState,
    #[serde(skip)]
    label: String,
    #[serde(skip)]
    value: f32,
}

impl Default for CjjTimer {
    fn default() -> Self {
        Self {
            start_non_engaged_instant: Instant::now(),
            total_non_engaged_duration: Duration::from_secs(0),
            start_regulation_instant: Instant::now(),
            total_regulation_duration: Duration::from_secs(0),
            regulation_input: MT,
            regulation_duration: Duration::from_secs(MT),
            penalty_free_input: PFT,
            penalty_free_duration: Duration::from_secs(PFT),
            total_penalty_duration: Duration::from_secs(0),
            half_penalty_free_duration: Duration::from_secs(HPFT),
            over_time: false,
            state: RegulationState::None,
            label: "Hello World!".to_owned(),
            value: 2.7,
        }
    }
}

impl CjjTimer {
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
#[derive(Debug, Clone, Copy, PartialEq)]
enum RegulationState {
    Start,
    Restarted,
    NotEngaged,
    Paused,
    Engaged,
    Overtime,
    End,
    None,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
enum RegulationTransition {
    Start,
    Restart,
    Pause,
    Engage,
    Separate,
    TimeExpire,
    Submission,
}

impl CjjTimer {
    fn change_regulation_state(&self, event: RegulationTransition) -> RegulationState {
        match (self.state, event) {
            (RegulationState::None, RegulationTransition::Start) => RegulationState::Start,
            (RegulationState::Restarted, RegulationTransition::Start) => RegulationState::Start,

            (RegulationState::Start, RegulationTransition::Separate)
            | (RegulationState::Paused, RegulationTransition::Separate)
            | (RegulationState::Engaged, RegulationTransition::Separate) => {
                RegulationState::NotEngaged
            }

            (RegulationState::NotEngaged, RegulationTransition::Engage) => RegulationState::Engaged,

            (RegulationState::Engaged, RegulationTransition::Pause)
            | (RegulationState::NotEngaged, RegulationTransition::Pause) => RegulationState::Paused,

            (RegulationState::Engaged, RegulationTransition::Submission) => RegulationState::End,

            (RegulationState::Engaged, RegulationTransition::TimeExpire)
            | (RegulationState::NotEngaged, RegulationTransition::TimeExpire) => {
                RegulationState::Overtime
            }

            (RegulationState::End, RegulationTransition::Restart) => RegulationState::Restarted,
            (RegulationState::Overtime, RegulationTransition::Restart) => {
                RegulationState::Restarted
            }
            _ => RegulationState::None,
        }
    }

    fn input(&self, event: RegulationTransition) -> RegulationTransition {
        if let RegulationState::Start = self.state {
            return RegulationTransition::Separate;
        }
        if let (true, s) = (self.over_time, self.state) {
            match s {
                RegulationState::Engaged => {
                    return RegulationTransition::TimeExpire;
                }
                RegulationState::NotEngaged => {
                    return RegulationTransition::TimeExpire;
                }
                _ => {
                    return event;
                }
            };
        }
        event
    }

    fn change(&mut self, event: RegulationTransition) {
        let new_state = self.change_regulation_state(self.input(event));
        match new_state {
            RegulationState::Start => {
                self.over_time = false;
                self.start_non_engaged_instant = Instant::now();
                self.total_non_engaged_duration = Duration::from_secs(0);
                self.start_regulation_instant = Instant::now();
                self.total_regulation_duration = Duration::from_secs(0);
                self.total_penalty_duration = Duration::from_secs(0);
                self.half_penalty_free_duration = Duration::from_secs(self.penalty_free_input / 2);
            }
            RegulationState::NotEngaged => {
                if RegulationState::Paused != self.state {
                    self.total_regulation_duration += self.start_regulation_instant.elapsed();
                }
                self.start_regulation_instant = Instant::now();
                self.start_non_engaged_instant = Instant::now();
            }
            RegulationState::Engaged => {
                self.total_regulation_duration += self.start_regulation_instant.elapsed();
                self.total_non_engaged_duration += self.start_non_engaged_instant.elapsed();
                self.start_regulation_instant = Instant::now();
                self.start_non_engaged_instant = Instant::now();
            }
            RegulationState::Paused => {
                if RegulationState::Engaged == self.state {
                    self.total_regulation_duration += self.start_regulation_instant.elapsed();
                    self.start_non_engaged_instant = Instant::now();
                    self.start_regulation_instant = Instant::now();
                }
                if RegulationState::NotEngaged == self.state {
                    self.total_regulation_duration += self.start_regulation_instant.elapsed();
                    self.total_non_engaged_duration += self.start_non_engaged_instant.elapsed();
                    self.start_non_engaged_instant = Instant::now();
                    self.start_regulation_instant = Instant::now();
                }
            }
            RegulationState::Overtime => {
                self.total_regulation_duration += self.start_regulation_instant.elapsed();
                if RegulationState::NotEngaged == self.state {
                    self.total_non_engaged_duration += self.start_non_engaged_instant.elapsed();
                }
                if self.total_non_engaged_duration > self.penalty_free_duration {
                    self.total_penalty_duration =
                        self.total_non_engaged_duration - self.penalty_free_duration;
                }
            }
            RegulationState::Restarted => {}
            RegulationState::End => {}
            RegulationState::None => {}
        }
        self.state = new_state;
    }
}

fn integer_edit_field(
    ui: &mut egui::Ui,
    value: &mut u64,
    duration: &mut Duration,
) -> egui::Response {
    let mut tmp_value = format!("{:?}", value);
    let res = ui.text_edit_singleline(&mut tmp_value);
    if let Ok(result) = tmp_value.parse() {
        *value = result;
        *duration = Duration::from_secs(*value);
    }
    res
}
fn format_time(label: &str, duration: Duration) -> String {
    let seconds = duration.as_secs() % 60;
    let minutes = (duration.as_secs() / 60) % 60;
    format!("{}: {:02}:{:02}", label, minutes, seconds)
}

impl eframe::App for CjjTimer {
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
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::new(60.0, eframe::epaint::FontFamily::Proportional),
            );
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(30.0, eframe::epaint::FontFamily::Proportional),
            );
            ui.heading("Hong Kong Combat Jiu Jitsu Timer");
            let current_non_engaged_time = match self.state {
                RegulationState::NotEngaged => {
                    self.total_non_engaged_duration + self.start_non_engaged_instant.elapsed()
                }
                _ => self.total_non_engaged_duration,
            };
            let match_stage = if (current_non_engaged_time >= Duration::from_secs(0))
                && (current_non_engaged_time < self.half_penalty_free_duration)
            {
                MatchStage::FirstHalfPenaltyFree
            } else if (current_non_engaged_time >= self.half_penalty_free_duration)
                && (current_non_engaged_time < self.penalty_free_duration)
            {
                MatchStage::SecondHalfPenaltyFree
            } else {
                MatchStage::Penalty
            };
            match self.state {
                RegulationState::Start => {
                    self.change(RegulationTransition::Separate);
                }
                RegulationState::NotEngaged => {
                    if (self.total_regulation_duration + self.start_regulation_instant.elapsed())
                        >= self.regulation_duration
                    {
                        self.change(RegulationTransition::TimeExpire);
                    }
                    ui.label("Fighters are NOT ENGAGED".to_string());
                    ui.label(format_time(
                        "Match Time",
                        self.total_regulation_duration + self.start_regulation_instant.elapsed(),
                    ));
                    match match_stage {
                        MatchStage::FirstHalfPenaltyFree => {
                            ui.colored_label(
                                egui::Color32::GREEN,
                                format_time(
                                    "1st Penalty Free Time",
                                    self.total_non_engaged_duration
                                        + self.start_non_engaged_instant.elapsed(),
                                ),
                            );
                        }
                        MatchStage::SecondHalfPenaltyFree => {
                            ui.colored_label(
                                egui::Color32::KHAKI,
                                format_time(
                                    "2nd Penalty Free Time",
                                    self.total_non_engaged_duration
                                        + self.start_non_engaged_instant.elapsed(),
                                ),
                            );
                        }
                        MatchStage::Penalty => {
                            ui.colored_label(
                                egui::Color32::RED,
                                format_time(
                                    "Penalty Time",
                                    self.total_non_engaged_duration
                                        + self.start_non_engaged_instant.elapsed()
                                        - self.penalty_free_duration,
                                ),
                            );
                        }
                    };
                    if ui.button("Engaged").clicked() {
                        self.change(RegulationTransition::Engage);
                    }

                    ui.separator();
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        if ui.button("Pause").clicked() {
                            self.change(RegulationTransition::Pause);
                        }
                    });
                }
                RegulationState::Engaged => {
                    if (self.total_regulation_duration + self.start_regulation_instant.elapsed())
                        >= self.regulation_duration
                    {
                        self.change(RegulationTransition::TimeExpire);
                    }
                    ui.label("Fighters are ENGAGED".to_string());
                    ui.label(format_time(
                        "Match Time",
                        self.total_regulation_duration + self.start_regulation_instant.elapsed(),
                    ));
                    match match_stage {
                        MatchStage::FirstHalfPenaltyFree => {
                            ui.colored_label(
                                egui::Color32::GREEN,
                                format_time(
                                    "1st Penalty Free Time",
                                    self.total_non_engaged_duration,
                                ),
                            );
                        }
                        MatchStage::SecondHalfPenaltyFree => {
                            ui.colored_label(
                                egui::Color32::KHAKI,
                                format_time(
                                    "2nd Penalty Free Time",
                                    self.total_non_engaged_duration,
                                ),
                            );
                        }
                        MatchStage::Penalty => {
                            ui.colored_label(
                                egui::Color32::RED,
                                format_time(
                                    "Penalty Time",
                                    self.total_non_engaged_duration - self.penalty_free_duration,
                                ),
                            );
                        }
                    };
                    if ui.button("Not Engaged").clicked() {
                        self.change(RegulationTransition::Separate);
                    }
                    ui.separator();
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        if ui.button("Pause").clicked() {
                            self.change(RegulationTransition::Pause);
                        }
                        if ui.button("Submission").clicked() {
                            self.change(RegulationTransition::Submission);
                        }
                    });
                }
                RegulationState::Paused => {
                    ui.label("Match is PAUSED".to_string());
                    if ui.button("Not Engaged").clicked() {
                        self.change(RegulationTransition::Separate);
                    }
                }
                RegulationState::Overtime => {
                    ui.label("Match is OVERTIME".to_string());
                    ui.label(format_time(
                        "Total Match Time",
                        self.total_regulation_duration,
                    ));
                    ui.label(format_time("Penalty Time", self.total_penalty_duration));
                    ui.separator();
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        if ui.button("Restart").clicked() {
                            self.change(RegulationTransition::Restart);
                        }
                    });
                }
                RegulationState::Restarted => {
                    ui.label("Match is RESTARTED".to_string());
                    if ui.button("Start").clicked() {
                        self.change(RegulationTransition::Start);
                    }
                }
                RegulationState::End => {
                    ui.label("Match ended in SUBMISSION".to_string());
                    if ui.button("Restart").clicked() {
                        self.change(RegulationTransition::Restart);
                    }
                }
                RegulationState::None => {
                    integer_edit_field(
                        ui,
                        &mut self.regulation_input,
                        &mut self.regulation_duration,
                    );
                    integer_edit_field(
                        ui,
                        &mut self.penalty_free_input,
                        &mut self.penalty_free_duration,
                    );
                    ui.label(format_time("Match Time", self.regulation_duration));
                    ui.label(format_time(
                        "Penalty Free Time",
                        self.total_penalty_duration,
                    ));
                    if ui.button("Start").clicked() {
                        self.change(RegulationTransition::Start);
                    }
                }
            }
        });
        ctx.request_repaint_after(Duration::from_secs(1));
    }
}
