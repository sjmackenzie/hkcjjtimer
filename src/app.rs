//use core::time::Duration;
//use std::time::Instant;
use web_time::{Duration, Instant};

const MT: u64 = 20;
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
    regulation_duration: Duration,
    #[serde(skip)]
    penalty_free_duration: Duration,
    #[serde(skip)]
    half_penalty_free_duration: Duration,
    #[serde(skip)]
    total_penalty_duration: Duration,
    #[serde(skip)]
    over_time: bool,
    #[serde(skip)]
    state: State,
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
            regulation_duration: Duration::from_secs(MT),
            penalty_free_duration: Duration::from_secs(PFT),
            total_penalty_duration: Duration::from_secs(0),
            half_penalty_free_duration: Duration::from_secs(HPFT),
            over_time: false,
            state: State::None,
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
enum State {
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
enum Event {
    Start,
    Restart,
    Pause,
    Engage,
    Separate,
    TimeExpire,
    Submission,
}

impl CjjTimer {
    fn change_regulation_state(&self, event: Event) -> State {
        match (self.state, event) {
            (State::None, Event::Start) => State::Start,
            (State::Restarted, Event::Start) => State::Start,

            (State::Start, Event::Separate)
            | (State::Paused, Event::Separate)
            | (State::Engaged, Event::Separate) => State::NotEngaged,

            (State::NotEngaged, Event::Engage) => State::Engaged,

            (State::Engaged, Event::Pause) | (State::NotEngaged, Event::Pause) => State::Paused,

            (State::Engaged, Event::Submission) => State::End,

            (State::Engaged, Event::TimeExpire) | (State::NotEngaged, Event::TimeExpire) => {
                State::Overtime
            }

            (State::End, Event::Restart) => State::Restarted,
            (State::Overtime, Event::Restart) => State::Restarted,
            _ => State::None,
        }
    }

    fn input(&self, event: Event) -> Event {
        if let State::Start = self.state {
            return Event::Separate;
        }
        if let (true, s) = (self.over_time, self.state) {
            match s {
                State::Engaged => {
                    return Event::TimeExpire;
                }
                State::NotEngaged => {
                    return Event::TimeExpire;
                }
                _ => {
                    return event;
                }
            };
        }
        event
    }

    fn change(&mut self, event: Event) {
        let new_state = self.change_regulation_state(self.input(event));
        match new_state {
            State::Start => {
                self.over_time = false;
                self.start_non_engaged_instant = Instant::now();
                self.total_non_engaged_duration = Duration::from_secs(0);
                self.start_regulation_instant = Instant::now();
                self.total_regulation_duration = Duration::from_secs(0);
                self.regulation_duration = Duration::from_secs(MT);
                self.penalty_free_duration = Duration::from_secs(PFT);
                self.total_penalty_duration = Duration::from_secs(0);
            }
            State::NotEngaged => {
                if State::Paused != self.state {
                    self.total_regulation_duration += self.start_regulation_instant.elapsed();
                }
                self.start_regulation_instant = Instant::now();
                self.start_non_engaged_instant = Instant::now();
            }
            State::Engaged => {
                self.total_regulation_duration += self.start_regulation_instant.elapsed();
                self.total_non_engaged_duration += self.start_non_engaged_instant.elapsed();
                self.start_regulation_instant = Instant::now();
                self.start_non_engaged_instant = Instant::now();
            }
            State::Paused => {
                if State::Engaged == self.state {
                    self.total_regulation_duration += self.start_regulation_instant.elapsed();
                    self.start_non_engaged_instant = Instant::now();
                    self.start_regulation_instant = Instant::now();
                }
                if State::NotEngaged == self.state {
                    self.total_regulation_duration += self.start_regulation_instant.elapsed();
                    self.total_non_engaged_duration += self.start_non_engaged_instant.elapsed();
                    self.start_non_engaged_instant = Instant::now();
                    self.start_regulation_instant = Instant::now();
                }
            }
            State::Overtime => {
                self.total_regulation_duration += self.start_regulation_instant.elapsed();
                if State::NotEngaged == self.state {
                    self.total_non_engaged_duration += self.start_non_engaged_instant.elapsed();
                }
                if self.total_non_engaged_duration > self.penalty_free_duration {
                    self.total_penalty_duration =
                        self.total_non_engaged_duration - self.penalty_free_duration;
                }
            }
            State::Restarted => {}
            State::End => {}
            State::None => {}
        }
        self.state = new_state;
    }
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
                State::NotEngaged => {
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
                State::Start => {
                    self.change(Event::Separate);
                }
                State::NotEngaged => {
                    if (self.total_regulation_duration + self.start_regulation_instant.elapsed())
                        >= self.regulation_duration
                    {
                        self.change(Event::TimeExpire);
                    }
                    ui.label("Fighters are NOT ENGAGED".to_string());
                    ui.label(format!(
                        "Match Time: {:?}",
                        (self.total_regulation_duration + self.start_regulation_instant.elapsed())
                            .as_secs()
                    ));
                    match match_stage {
                        MatchStage::FirstHalfPenaltyFree => {
                            ui.colored_label(
                                egui::Color32::GREEN,
                                format!(
                                    "First Half Penalty Free Time: {}",
                                    (self.total_non_engaged_duration
                                        + self.start_non_engaged_instant.elapsed())
                                    .as_secs()
                                ),
                            );
                        }
                        MatchStage::SecondHalfPenaltyFree => {
                            ui.colored_label(
                                egui::Color32::KHAKI,
                                format!(
                                    "Second Half Penalty Free Time: {}",
                                    (self.total_non_engaged_duration
                                        + self.start_non_engaged_instant.elapsed())
                                    .as_secs()
                                ),
                            );
                        }
                        MatchStage::Penalty => {
                            ui.colored_label(
                                egui::Color32::RED,
                                format!(
                                    "Penalty Time: {}",
                                    (self.total_non_engaged_duration
                                        + self.start_non_engaged_instant.elapsed()
                                        - self.penalty_free_duration)
                                        .as_secs()
                                ),
                            );
                        }
                    };
                    if ui.button("Engaged").clicked() {
                        self.change(Event::Engage);
                    }

                    ui.separator();
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        if ui.button("Pause").clicked() {
                            self.change(Event::Pause);
                        }
                    });
                }
                State::Engaged => {
                    if (self.total_regulation_duration + self.start_regulation_instant.elapsed())
                        >= self.regulation_duration
                    {
                        self.change(Event::TimeExpire);
                    }
                    ui.label("Fighters are ENGAGED".to_string());
                    ui.label(format!(
                        "Match Time: {:?}",
                        (self.total_regulation_duration + self.start_regulation_instant.elapsed())
                            .as_secs()
                    ));
                    match match_stage {
                        MatchStage::FirstHalfPenaltyFree => {
                            ui.colored_label(
                                egui::Color32::GREEN,
                                format!(
                                    "First Half Penalty Free Time: {}",
                                    self.total_non_engaged_duration.as_secs()
                                ),
                            );
                        }
                        MatchStage::SecondHalfPenaltyFree => {
                            ui.colored_label(
                                egui::Color32::KHAKI,
                                format!(
                                    "Second Half Penalty Free Time: {}",
                                    self.total_non_engaged_duration.as_secs()
                                ),
                            );
                        }
                        MatchStage::Penalty => {
                            ui.colored_label(
                                egui::Color32::RED,
                                format!(
                                    "Penalty Time: {}",
                                    (self.total_non_engaged_duration - self.penalty_free_duration)
                                        .as_secs()
                                ),
                            );
                        }
                    };
                    if ui.button("Not Engaged").clicked() {
                        self.change(Event::Separate);
                    }
                    ui.separator();
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        if ui.button("Pause").clicked() {
                            self.change(Event::Pause);
                        }
                        if ui.button("Submission").clicked() {
                            self.change(Event::Submission);
                        }
                    });
                }
                State::Paused => {
                    ui.label("Match is PAUSED".to_string());
                    if ui.button("Not Engaged").clicked() {
                        self.change(Event::Separate);
                    }
                }
                State::Overtime => {
                    ui.label("Match is OVERTIME".to_string());
                    ui.label(format!(
                        "Total Match Time: {:?}",
                        (self.total_regulation_duration)
                    ));

                    ui.label(format!("Penalty Time: {:?}", self.total_penalty_duration));
                    ui.separator();
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        if ui.button("Restart").clicked() {
                            self.change(Event::Restart);
                        }
                    });
                }
                State::Restarted => {
                    ui.label("Match is RESTARTED".to_string());
                    if ui.button("Start").clicked() {
                        self.change(Event::Start);
                    }
                }
                State::End => {
                    ui.label("Match ended in SUBMISSION".to_string());
                    if ui.button("Restart").clicked() {
                        self.change(Event::Restart);
                    }
                }
                State::None => {
                    ui.label(format!(
                        "Total Match Time: {:?}",
                        (self.total_regulation_duration).as_secs()
                    ));
                    ui.label(format!(
                        "Total Overtime: {:?}",
                        (self.total_non_engaged_duration).as_secs()
                    ));
                    if ui.button("Start").clicked() {
                        self.change(Event::Start);
                    }
                }
            }
        });
        ctx.request_repaint_after(Duration::from_secs(1));
    }
}
