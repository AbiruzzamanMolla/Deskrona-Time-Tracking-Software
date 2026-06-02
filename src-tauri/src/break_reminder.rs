use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

use crate::db;

/// Break reminder states
/// 0 = disabled/idle, 1 = counting down, 2 = pre-break warning, 3 = on break, 4 = paused
const STATE_IDLE: u8 = 0;
const STATE_COUNTING: u8 = 1;
const STATE_PRE_BREAK: u8 = 2;
const STATE_ON_BREAK: u8 = 3;
const STATE_PAUSED: u8 = 4;

static BREAK_STATE: AtomicU8 = AtomicU8::new(STATE_IDLE);
static BREAK_IS_LONG: AtomicBool = AtomicBool::new(false);
static BREAK_COUNTDOWN_SECS: AtomicI32 = AtomicI32::new(0);
static BREAK_MINI_COMPLETED: AtomicI32 = AtomicI32::new(0);
static BREAK_POSTPONE_COUNT: AtomicI32 = AtomicI32::new(0);
static BREAK_PAUSED_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);

/// Current idea being displayed during break
static CURRENT_BREAK_IDEA: Mutex<Option<String>> = Mutex::new(None);

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BreakReminderState {
    pub enabled: bool,
    pub state: String,            // "idle", "counting", "pre_break", "on_break", "paused"
    pub is_long: bool,
    pub countdown_secs: i32,
    pub mini_completed: i32,
    pub postpone_count: i32,
    pub postpone_limit: i32,
    pub current_idea: Option<String>,
}

/// Break ideas library - shown during breaks
pub const BREAK_IDEAS: &[&str] = &[
    "🧘 Stand up and stretch your arms above your head",
    "👀 Look away from your screen at something 20 feet away for 20 seconds",
    "💧 Take a sip of water",
    "🫁 Take 5 deep breaths - inhale for 4 seconds, exhale for 6 seconds",
    "🚶 Walk around your room for a minute",
    "🤸 Roll your shoulders forward and backward 10 times each",
    "😊 Smile! It releases endorphins and reduces stress",
    "🌿 Look out a window and focus on something green",
    "👋 Shake out your hands and wrists to release tension",
    "🧠 Close your eyes and think of 3 things you're grateful for today",
    "💪 Do 10 desk push-ups or wall push-ups",
    "🔄 Rotate your neck slowly in circles, 5 times each direction",
    "🦶 Point your toes up and down 20 times to improve circulation",
    "🎵 Listen to a favorite song snippet with your eyes closed",
    "📱 Put your phone away for the next break and be present",
    "🪑 Check your posture - sit up straight, feet flat on the floor",
    "🌻 Take a moment to appreciate your workspace",
    "👃 Close your eyes and identify 3 things you can smell",
    "🤲 Press your palms together firmly for 10 seconds, then release",
    "📝 Write down one thing you accomplished today so far",
    "🧘‍♂️ Do a quick 30-second meditation - focus on your breathing",
    "💺 Stand up and do 5 squats",
    "👁️ Do the 20-20-20 rule: every 20 min, look at something 20 feet away for 20 sec",
    "🎨 Look at something colorful - it refreshes your visual processing",
    "🗣️ Say something positive to yourself or a colleague",
    "🦴 Stretch your fingers wide, hold for 5 seconds, then make a fist. Repeat 5 times",
    "🌞 If possible, step outside for a breath of fresh air",
    "☕ Refill your water bottle or make a healthy drink",
    "🧹 Organize one small area of your desk",
    "🫶 Give yourself a gentle hand massage to relieve tension",
];

fn pick_random_idea() -> String {
    let idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as usize)
        % BREAK_IDEAS.len();
    BREAK_IDEAS[idx].to_string()
}

pub fn get_break_state(app: &AppHandle) -> BreakReminderState {
    let enabled = db::get_settings(app)
        .map(|s| s.break_reminder_enabled)
        .unwrap_or(false);
    let state_id = BREAK_STATE.load(Ordering::SeqCst);
    let state = match state_id {
        STATE_IDLE => "idle",
        STATE_COUNTING => "counting",
        STATE_PRE_BREAK => "pre_break",
        STATE_ON_BREAK => "on_break",
        STATE_PAUSED => "paused",
        _ => "idle",
    };
    let idea = CURRENT_BREAK_IDEA.lock().unwrap().clone();
    let postpone_limit = db::get_settings(app)
        .map(|s| s.break_postpone_limit)
        .unwrap_or(3);

    BreakReminderState {
        enabled,
        state: state.to_string(),
        is_long: BREAK_IS_LONG.load(Ordering::SeqCst),
        countdown_secs: BREAK_COUNTDOWN_SECS.load(Ordering::SeqCst),
        mini_completed: BREAK_MINI_COMPLETED.load(Ordering::SeqCst),
        postpone_count: BREAK_POSTPONE_COUNT.load(Ordering::SeqCst),
        postpone_limit,
        current_idea: idea,
    }
}

pub fn start_break_reminder(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_tick = Instant::now();

        loop {
            std::thread::sleep(Duration::from_secs(1));
            let now = Instant::now();
            let elapsed = now.duration_since(last_tick).as_secs() as i32;
            last_tick = now;

            let current_state = BREAK_STATE.load(Ordering::SeqCst);
            let tracking_status = crate::tracking::get_tracking_status();

            let settings = match db::get_settings(&app) {
                Ok(s) => s,
                Err(_) => continue,
            };

            if !settings.break_reminder_enabled {
                if current_state != STATE_IDLE {
                    BREAK_STATE.store(STATE_IDLE, Ordering::SeqCst);
                }
                continue;
            }

            // Check pause until
            {
                let pause_guard = BREAK_PAUSED_UNTIL.lock().unwrap();
                if let Some(pause_until) = *pause_guard {
                    if now < pause_until {
                        let current = BREAK_STATE.load(Ordering::SeqCst);
                        if current != STATE_PAUSED {
                            BREAK_STATE.store(STATE_PAUSED, Ordering::SeqCst);
                        }
                        continue;
                    }
                }
            }

            if tracking_status != "running" && current_state == STATE_COUNTING {
                BREAK_STATE.store(STATE_IDLE, Ordering::SeqCst);
                continue;
            }

            // Skip breaks during pomodoro focus phase
            let pomodoro_phase = crate::POMODORO_PHASE.lock().unwrap().clone();
            if pomodoro_phase == "focus" && current_state == STATE_COUNTING {
                BREAK_STATE.store(STATE_IDLE, Ordering::SeqCst);
                continue;
            }

            match current_state {
                STATE_IDLE => {
                    // Start counting down for the next mini break
                    let interval_secs = settings.break_mini_interval_minutes * 60;
                    BREAK_COUNTDOWN_SECS.store(interval_secs, Ordering::SeqCst);
                    BREAK_IS_LONG.store(false, Ordering::SeqCst);
                    BREAK_STATE.store(STATE_COUNTING, Ordering::SeqCst);
                }
                STATE_COUNTING => {
                    let remaining = BREAK_COUNTDOWN_SECS.load(Ordering::SeqCst) - elapsed;
                    if remaining <= 0 {
                        // Check if we should do a long break
                        let mini_done = BREAK_MINI_COMPLETED.load(Ordering::SeqCst);
                        let is_long = mini_done + 1 >= settings.break_mini_breaks_before_long;
                        BREAK_IS_LONG.store(is_long, Ordering::SeqCst);

                        if is_long {
                            // Go straight to long break
                            let duration = settings.break_long_duration_seconds;
                            BREAK_COUNTDOWN_SECS.store(duration, Ordering::SeqCst);
                        } else {
                            // Show pre-break warning first
                            let pre_warn = settings.break_pre_notification_seconds;
                            BREAK_COUNTDOWN_SECS.store(pre_warn, Ordering::SeqCst);
                        }
                        BREAK_POSTPONE_COUNT.store(0, Ordering::SeqCst);
                        let idea = pick_random_idea();
                        *CURRENT_BREAK_IDEA.lock().unwrap() = Some(idea.clone());
                        BREAK_STATE.store(STATE_PRE_BREAK, Ordering::SeqCst);
                        let _ = app.emit("break-pre-notification", &idea);
                    } else {
                        BREAK_COUNTDOWN_SECS.store(remaining, Ordering::SeqCst);
                    }
                }
                STATE_PRE_BREAK => {
                    let remaining = BREAK_COUNTDOWN_SECS.load(Ordering::SeqCst) - elapsed;
                    if remaining <= 0 {
                        // Start the actual break
                        let is_long = BREAK_IS_LONG.load(Ordering::SeqCst);
                        let duration = if is_long {
                            settings.break_long_duration_seconds
                        } else {
                            settings.break_mini_duration_seconds
                        };
                        BREAK_COUNTDOWN_SECS.store(duration, Ordering::SeqCst);
                        BREAK_STATE.store(STATE_ON_BREAK, Ordering::SeqCst);
                        let idea = CURRENT_BREAK_IDEA.lock().unwrap().clone().unwrap_or_default();
                        let _ = app.emit("break-started", &idea);
                    } else {
                        BREAK_COUNTDOWN_SECS.store(remaining, Ordering::SeqCst);
                    }
                }
                STATE_ON_BREAK => {
                    let remaining = BREAK_COUNTDOWN_SECS.load(Ordering::SeqCst) - elapsed;
                    if remaining <= 0 {
                        // Break finished
                                                finish_break(&app);
                    } else {
                        BREAK_COUNTDOWN_SECS.store(remaining, Ordering::SeqCst);
                    }
                }
                STATE_PAUSED => {
                    // Check if pause expired
                    let pause_guard = BREAK_PAUSED_UNTIL.lock().unwrap();
                    if let Some(pause_until) = *pause_guard {
                        if now >= pause_until {
                            drop(pause_guard);
                            BREAK_STATE.store(STATE_IDLE, Ordering::SeqCst);
                            let _ = app.emit("break-resumed", ());
                        }
                    }
                }
                _ => {}
            }
        }
    });
}

fn finish_break(app: &AppHandle) {
    let is_long = BREAK_IS_LONG.load(Ordering::SeqCst);

    if is_long {
        // Reset mini count after a long break
        BREAK_MINI_COMPLETED.store(0, Ordering::SeqCst);
    } else {
        BREAK_MINI_COMPLETED.fetch_add(1, Ordering::SeqCst);
    }

    BREAK_STATE.store(STATE_IDLE, Ordering::SeqCst);
    *CURRENT_BREAK_IDEA.lock().unwrap() = None;
    let _ = app.emit("break-finished", ());
}

pub fn postpone_break(app: &AppHandle) -> Result<BreakReminderState, String> {
    let current = BREAK_STATE.load(Ordering::SeqCst);
    if current != STATE_ON_BREAK && current != STATE_PRE_BREAK {
        return Err("No active break to postpone".to_string());
    }

    let settings = db::get_settings(app).map_err(|e| e.to_string())?;
    let count = BREAK_POSTPONE_COUNT.load(Ordering::SeqCst);
    if count >= settings.break_postpone_limit {
        return Err(format!(
            "Postpone limit reached ({})",
            settings.break_postpone_limit
        ));
    }

    BREAK_POSTPONE_COUNT.fetch_add(1, Ordering::SeqCst);

    // Start counting again with postpone duration
    let postpone_secs = settings.break_postpone_duration_minutes * 60;
    BREAK_COUNTDOWN_SECS.store(postpone_secs, Ordering::SeqCst);
    BREAK_STATE.store(STATE_COUNTING, Ordering::SeqCst);

    let _ = app.emit("break-postponed", ());
    Ok(get_break_state(app))
}

pub fn skip_break(app: &AppHandle) -> Result<BreakReminderState, String> {
    let current = BREAK_STATE.load(Ordering::SeqCst);
    if current == STATE_IDLE || current == STATE_PAUSED {
        return Err("No active break to skip".to_string());
    }

    let _ = db::get_settings(app).map_err(|e| e.to_string())?;
    finish_break(app);
    Ok(get_break_state(app))
}

pub fn pause_break_reminder(app: &AppHandle, duration_minutes: i32) -> Result<BreakReminderState, String> {
    if duration_minutes <= 0 {
        return Err("Invalid pause duration".to_string());
    }

    let pause_until = Instant::now() + Duration::from_secs((duration_minutes * 60) as u64);
    {
        let mut pause_guard = BREAK_PAUSED_UNTIL.lock().unwrap();
        *pause_guard = Some(pause_until);
    }

    // If currently on a break, skip it first
    let current = BREAK_STATE.load(Ordering::SeqCst);
    if current == STATE_ON_BREAK || current == STATE_PRE_BREAK {
        finish_break(app);
    }

    BREAK_STATE.store(STATE_PAUSED, Ordering::SeqCst);
    let _ = app.emit("break-paused", duration_minutes);
    Ok(get_break_state(app))
}

pub fn resume_break_reminder(app: &AppHandle) -> Result<BreakReminderState, String> {
    {
        let mut pause_guard = BREAK_PAUSED_UNTIL.lock().unwrap();
        *pause_guard = None;
    }
    BREAK_STATE.store(STATE_IDLE, Ordering::SeqCst);
    let _ = app.emit("break-resumed", ());
    Ok(get_break_state(app))
}

pub fn reset_break_cycle(app: &AppHandle) -> Result<BreakReminderState, String> {
    BREAK_STATE.store(STATE_IDLE, Ordering::SeqCst);
    BREAK_MINI_COMPLETED.store(0, Ordering::SeqCst);
    BREAK_POSTPONE_COUNT.store(0, Ordering::SeqCst);
    BREAK_COUNTDOWN_SECS.store(0, Ordering::SeqCst);
    *CURRENT_BREAK_IDEA.lock().unwrap() = None;
    {
        let mut pause_guard = BREAK_PAUSED_UNTIL.lock().unwrap();
        *pause_guard = None;
    }
    let _ = app.emit("break-reset", ());
    Ok(get_break_state(app))
}

// ─── Tauri Commands ──────────────────────────────────────────────

#[tauri::command]
pub fn cmd_break_status(app: tauri::AppHandle) -> Result<BreakReminderState, String> {
    Ok(get_break_state(&app))
}

#[tauri::command]
pub fn cmd_break_postpone(app: tauri::AppHandle) -> Result<BreakReminderState, String> {
    postpone_break(&app)
}

#[tauri::command]
pub fn cmd_break_skip(app: tauri::AppHandle) -> Result<BreakReminderState, String> {
    skip_break(&app)
}

#[tauri::command]
pub fn cmd_break_pause(app: tauri::AppHandle, duration_minutes: i32) -> Result<BreakReminderState, String> {
    pause_break_reminder(&app, duration_minutes)
}

#[tauri::command]
pub fn cmd_break_resume(app: tauri::AppHandle) -> Result<BreakReminderState, String> {
    resume_break_reminder(&app)
}

#[tauri::command]
pub fn cmd_break_reset(app: tauri::AppHandle) -> Result<BreakReminderState, String> {
    reset_break_cycle(&app)
}
