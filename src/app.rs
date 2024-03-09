use web_time::{Duration, Instant};

const MT: u64 = 600;
const PFT: u64 = 90;
const HPFT: u64 = PFT / 2;
const SOT: u64 = 120;

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
    total_overtime_duration: Duration,
    #[serde(skip)]
    start_overtime_instant: Instant,
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
    penalty_time_divided: Duration,
    #[serde(skip)]
    standard_overtime_duration: Duration,
    #[serde(skip)]
    over_time: bool,
    #[serde(skip)]
    state: RegulationState,
    #[serde(skip)]
    overtime_state: OvertimeState,
    #[serde(skip)]
    overtime_segments: Vec<Segment>,
    #[serde(skip)]
    winner: Option<Fighter>,
}

#[derive(Debug)]
enum Fighter {
    A,
    B,
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
            penalty_time_divided: Duration::from_secs(0),
            total_overtime_duration: Duration::from_secs(0),
            start_overtime_instant: Instant::now(),
            half_penalty_free_duration: Duration::from_secs(HPFT),
            standard_overtime_duration: Duration::from_secs(SOT),
            over_time: false,
            state: RegulationState::None,
            overtime_state: OvertimeState::AdvanceOvertime,
            overtime_segments: vec![],
            winner: None,
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
enum Segment {
    Escape(Duration),
    Submission(Duration),
}
#[derive(Debug, Clone, Copy, PartialEq)]
enum RegulationState {
    Start,
    Restarted,
    NotEngaged,
    Paused,
    Engaged,
    Overtime,
    Submission,
    None,
}

#[derive(Debug, Copy, Clone, serde::Deserialize, serde::Serialize)]
enum Transition {
    StartRegulation,
    StartOvertime,
    Restart,
    Pause,
    Engage,
    Separate,
    TimeExpire,
    Submission,
    Win,
}

#[derive(Copy, PartialEq, Clone)]
enum OvertimeState {
    Paused,
    Engaged,
    Escaped,
    Submission,
    AdvanceOvertime,
    Win,
}

impl CjjTimer {
    fn change_regulation_state(&self, event: Transition) -> RegulationState {
        match (self.state, event) {
            (RegulationState::None, Transition::StartRegulation) => RegulationState::Start,
            (RegulationState::Restarted, Transition::StartRegulation) => RegulationState::Start,

            (RegulationState::Start, Transition::Separate)
            | (RegulationState::Paused, Transition::Separate)
            | (RegulationState::Engaged, Transition::Separate) => RegulationState::NotEngaged,

            (RegulationState::NotEngaged, Transition::Engage) => RegulationState::Engaged,

            (RegulationState::Engaged, Transition::Pause)
            | (RegulationState::NotEngaged, Transition::Pause) => RegulationState::Paused,

            (RegulationState::Engaged, Transition::Submission) => RegulationState::Submission,

            (RegulationState::Engaged, Transition::TimeExpire)
            | (RegulationState::NotEngaged, Transition::TimeExpire) => RegulationState::Overtime,

            (RegulationState::Submission, Transition::Restart) => RegulationState::Restarted,

            _ => RegulationState::None,
        }
    }

    fn regulation_input(&self, event: Transition) -> Transition {
        if let RegulationState::Start = self.state {
            return Transition::Separate;
        }
        if let (true, s) = (self.over_time, self.state) {
            match s {
                RegulationState::Engaged => {
                    return Transition::TimeExpire;
                }
                RegulationState::NotEngaged => {
                    return Transition::TimeExpire;
                }
                _ => {
                    return event;
                }
            };
        }
        event
    }

    fn change_regulation(&mut self, event: Transition) {
        let new_state = self.change_regulation_state(self.regulation_input(event));
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
                    self.penalty_time_divided = self.total_penalty_duration / 2;
                }
                self.change_overtime(self.overtime_input(event));
            }
            RegulationState::Restarted => {}
            RegulationState::Submission => {}
            RegulationState::None => {}
        }
        self.state = new_state;
    }

    fn change_overtime_state(&self, event: Transition) -> OvertimeState {
        match (self.overtime_state, event) {
            (OvertimeState::AdvanceOvertime, Transition::Engage) => OvertimeState::Engaged,

            (OvertimeState::Engaged, Transition::Separate) => OvertimeState::Escaped,

            (OvertimeState::Engaged, Transition::Submission) => OvertimeState::Submission,

            (OvertimeState::Engaged, Transition::TimeExpire) => OvertimeState::AdvanceOvertime,

            (OvertimeState::Engaged, Transition::Win) => OvertimeState::Win,

            (OvertimeState::Engaged, Transition::Pause) => OvertimeState::Paused,

            (OvertimeState::Paused, Transition::Engage) => OvertimeState::Engaged,

            _ => OvertimeState::AdvanceOvertime,
        }
    }

    fn overtime_input(&self, event: Transition) -> Transition {
        event
    }
    fn change_overtime(&mut self, event: Transition) {
        let new_state = self.change_overtime_state(self.overtime_input(event));
        match new_state {
            OvertimeState::Engaged => {
                if OvertimeState::Paused != self.overtime_state {
                    self.total_overtime_duration += self.start_overtime_instant.elapsed();
                }
                self.start_overtime_instant = Instant::now();
            }
            OvertimeState::Paused => {
                if OvertimeState::Engaged == self.overtime_state {
                    self.total_overtime_duration += self.start_overtime_instant.elapsed();
                    self.start_overtime_instant = Instant::now();
                }
            }
            OvertimeState::AdvanceOvertime => {}
            OvertimeState::Submission => {}
            OvertimeState::Escaped => {}
            OvertimeState::Win => {}
        }
        self.overtime_state = new_state;
    }
    fn calculate_win(&mut self) -> bool {
        if self.overtime_segments.len() % 2 == 0 {
            let a: Segment = self.overtime_segments[self.overtime_segments.len() - 2];
            let b: Segment = self.overtime_segments[self.overtime_segments.len() - 1];
            match (a, b) {
                (Segment::Submission(t1), Segment::Submission(t2)) => {
                    if t1 < t2 {
                        self.winner = Some(Fighter::A);
                        true
                    } else {
                        self.winner = Some(Fighter::B);
                        true
                    }
                }
                (Segment::Submission(_), Segment::Escape(_)) => {
                    self.winner = Some(Fighter::A);
                    true
                }
                (Segment::Escape(_), Segment::Submission(_)) => {
                    self.winner = Some(Fighter::B);
                    true
                }
                (Segment::Escape(_), Segment::Escape(_)) => {
                    if self.overtime_segments.len() >= 6 {
                        let mut a: Duration = Duration::from_secs(0);
                        let mut b: Duration = Duration::from_secs(0);
                        for (i, segment) in self.overtime_segments.iter().enumerate() {
                            if i % 2 == 0 {
                                if let Segment::Escape(t) = segment {
                                    a += *t;
                                }
                            } else if let Segment::Escape(t) = segment {
                                b += *t;
                            }
                        }
                        //let a: Duration = self.overtime_segments.iter().enumerate().filter_map(|(index, value)| if index % 2 == 0 { Some(value) } else { None } ).sum();
                        //let b: Duration = self.overtime_segments.iter().enumerate().filter_map(|(index, value)| if index % 1 == 0 { Some(value) } else { None } ).sum();
                        if a == b {
                            return false;
                        }
                        if a > b {
                            self.winner = Some(Fighter::B);
                        } else {
                            self.winner = Some(Fighter::A);
                        }
                        return true;
                    }
                    false
                }
            }
        } else {
            false
        }
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
                    self.change_regulation(Transition::Separate);
                }
                RegulationState::NotEngaged => {
                    if (self.total_regulation_duration + self.start_regulation_instant.elapsed())
                        >= self.regulation_duration
                    {
                        self.change_regulation(Transition::TimeExpire);
                    }
                    ui.label("Fighters are NOT ENGAGED".to_string());
                    ui.label(format_time("Match Time", self.regulation_duration));
                    ui.label(format_time(
                        "Current Time",
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
                        self.change_regulation(Transition::Engage);
                    }

                    ui.separator();
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        if ui.button("Pause").clicked() {
                            self.change_regulation(Transition::Pause);
                        }
                    });
                }
                RegulationState::Engaged => {
                    if (self.total_regulation_duration + self.start_regulation_instant.elapsed())
                        >= self.regulation_duration
                    {
                        self.change_regulation(Transition::TimeExpire);
                    }
                    ui.label("Fighters are ENGAGED".to_string());
                    ui.label(format_time("Match Time", self.regulation_duration));
                    ui.label(format_time(
                        "Current Time",
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
                        self.change_regulation(Transition::Separate);
                    }
                    ui.separator();
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        if ui.button("Pause").clicked() {
                            self.change_regulation(Transition::Pause);
                        }
                        if ui.button("Submission").clicked() {
                            self.change_regulation(Transition::Submission);
                        }
                    });
                }
                RegulationState::Paused => {
                    ui.label("Match is PAUSED".to_string());
                    if ui.button("Not Engaged").clicked() {
                        self.change_regulation(Transition::Separate);
                    }
                }
                RegulationState::Overtime => {
                    ui.label("Match is OVERTIME".to_string());
                    match self.overtime_state {
                        OvertimeState::AdvanceOvertime => {
                            ui.label("Advance Overtime Round");
                            ui.label(format!("Escape Time: {:?}", self.overtime_segments));
                            if ui.button("Start Round").clicked() {
                                self.total_overtime_duration = Duration::from_secs(0);
                                self.start_overtime_instant = Instant::now();
                                self.change_overtime(Transition::Engage);
                            }
                        }
                        OvertimeState::Engaged => {
                            ui.label("Fighters are Engaged");
                            let current_ot = self.total_overtime_duration
                                + self.start_overtime_instant.elapsed();
                            let calculated_ot =
                                self.penalty_time_divided + self.standard_overtime_duration;
                            ui.label(format_time("Segment Time", calculated_ot));
                            ui.label(format_time("Current Time", current_ot));
                            if current_ot >= calculated_ot {
                                self.overtime_segments.push(Segment::Escape(calculated_ot));
                                if self.calculate_win() {
                                    self.change_overtime(Transition::Win);
                                } else {
                                    self.change_overtime(Transition::TimeExpire);
                                }
                            }
                            if ui.button("Escape").clicked() {
                                // addressing = 2 * element + segment
                                self.overtime_segments.push(Segment::Escape(
                                    self.total_overtime_duration
                                        + self.start_overtime_instant.elapsed(),
                                ));
                                if self.calculate_win() {
                                    self.change_overtime(Transition::Win);
                                } else {
                                    self.change_overtime(Transition::Separate);
                                }
                            }
                            if ui.button("Submission").clicked() {
                                self.overtime_segments.push(Segment::Submission(
                                    self.total_overtime_duration
                                        + self.start_overtime_instant.elapsed(),
                                ));
                                if self.calculate_win() {
                                    self.change_overtime(Transition::Win);
                                } else {
                                    self.change_overtime(Transition::Submission);
                                }
                            }
                            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                                if ui.button("Pause").clicked() {
                                    self.change_overtime(Transition::Pause);
                                }
                            });
                        }
                        OvertimeState::Escaped => {
                            ui.label("Fighters Escaped");
                            ui.label(format!("Rounds: {:?}", self.overtime_segments));
                            if ui.button("Advance Round").clicked() {
                                self.change_overtime(Transition::TimeExpire);
                            }
                        }
                        OvertimeState::Submission => {
                            ui.label("Fighter Submission");
                            ui.label(format!("Rounds: {:?}", self.overtime_segments));
                            if ui.button("Advance Round").clicked() {
                                self.change_overtime(Transition::TimeExpire);
                            }
                        }
                        OvertimeState::Paused => {
                            ui.label("Match is in Overtime Paused");
                            if ui.button("Engage").clicked() {
                                self.change_overtime(Transition::Engage);
                            }
                        }
                        OvertimeState::Win => {
                            ui.label("First round offensive is A".to_string());
                            ui.label("First round defensive is B".to_string());
                            ui.separator();
                            ui.label(format!(
                                "The Winner is: {:?}",
                                self.winner.as_ref().unwrap()
                            ));
                            for (i, segment) in self.overtime_segments.iter().enumerate() {
                                let fighter = if i % 2 == 0 { Fighter::A } else { Fighter::B };
                                ui.label(format!("{:?}{}: {:?}", fighter, i, segment));
                            }
                        }
                    };
                }
                RegulationState::Restarted => {
                    ui.label("Match is RESTARTED".to_string());
                    if ui.button("Start").clicked() {
                        self.change_regulation(Transition::StartRegulation);
                    }
                }
                RegulationState::Submission => {
                    ui.label("Match ended in SUBMISSION".to_string());
                    if ui.button("Restart").clicked() {
                        self.change_regulation(Transition::Restart);
                    }
                }
                RegulationState::None => {
                    integer_edit_field(
                        ui,
                        &mut self.regulation_input,
                        &mut self.regulation_duration,
                    );
                    ui.label("as seconds".to_string());
                    integer_edit_field(
                        ui,
                        &mut self.penalty_free_input,
                        &mut self.penalty_free_duration,
                    );
                    ui.label("as seconds".to_string());
                    ui.separator();
                    ui.label(format_time("Match Time", self.regulation_duration));
                    ui.label(format_time(
                        "Penalty Free Duration",
                        self.penalty_free_duration,
                    ));
                    if ui.button("Start").clicked() {
                        self.change_regulation(Transition::StartRegulation);
                    }
                }
            }
        });
        ctx.request_repaint_after(Duration::from_secs(1));
    }
}
