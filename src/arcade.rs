use std::sync::OnceLock;

use crate::actors::GhostKind;

const ORIGINAL_FPS: f32 = 60.606_06;
pub const ORIGINAL_FRAME_TIME: f32 = 1.0 / ORIGINAL_FPS;
pub const ARCADE_TIMER_TICKS_PER_SECOND: f32 = 120.0;
pub const FRIGHT_FLASH_START_TICKS: u16 = 0x0100;
pub const FRIGHT_FLASH_HALF_PERIOD_TICKS: u8 = 0x0e;
const ARCADE_RULES: &str = include_str!("../assets/arcade/arcade-rules.txt");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MovePatternState {
    base: u32,
    current: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArcadeLevelSpec {
    pub fruit_points: u32,
    pub pacman_speed: f32,
    pub frightened_pacman_speed: Option<f32>,
    pub ghost_speed: f32,
    pub ghost_tunnel_speed: f32,
    pub elroy_one_dots_left: usize,
    pub elroy_one_speed: f32,
    pub elroy_two_dots_left: usize,
    pub elroy_two_speed: f32,
    pub frightened_ghost_speed: Option<f32>,
    pub frightened_time: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArcadeMovePatterns {
    pub pacman_normal: u32,
    pub pacman_frightened: u32,
    pub blinky_elroy_two: u32,
    pub blinky_elroy_one: u32,
    pub ghost_normal: u32,
    pub ghost_frightened: u32,
    pub ghost_tunnel: u32,
}

#[derive(Clone, Debug)]
struct ArcadeRuleTables {
    difficulty_entries: [[u8; 6]; 21],
    personal_dot_groups: [[u8; 3]; 4],
    elroy_dot_groups: [[u8; 2]; 8],
    frightened_timer_ticks: [u16; 9],
    release_timer_frames: [u16; 3],
    move_pattern_groups: [[u32; 7]; 7],
    phase_change_frames: [[u16; 7]; 7],
    fruit_points: [u32; 21],
    fruit_reset_timer: u8,
    fruit_release_dots: [u8; 2],
    global_release_dots: [u8; 3],
}

impl MovePatternState {
    pub fn new(base: u32) -> Self {
        Self {
            base,
            current: base,
        }
    }

    pub fn reset(&mut self) {
        self.current = self.base;
    }

    pub fn advance(&mut self) -> bool {
        let move_now = (self.current & 0x8000_0000) != 0;
        self.current = self.current.rotate_left(1);
        move_now
    }

    pub fn base(&self) -> u32 {
        self.base
    }
}

pub fn level_spec(level: u32) -> ArcadeLevelSpec {
    let (elroy_one_dots_left, elroy_two_dots_left) = elroy_dot_limits(level);
    let frightened_time = frightened_time_seconds(level);
    let fruit_points = fruit_points(level);
    let patterns = move_patterns(level);
    let frightened_available = frightened_time > (1.0 / ARCADE_TIMER_TICKS_PER_SECOND);

    ArcadeLevelSpec {
        fruit_points,
        pacman_speed: pattern_speed(patterns.pacman_normal),
        frightened_pacman_speed: frightened_available
            .then_some(pattern_speed(patterns.pacman_frightened)),
        ghost_speed: pattern_speed(patterns.ghost_normal),
        ghost_tunnel_speed: pattern_speed(patterns.ghost_tunnel),
        elroy_one_dots_left,
        elroy_one_speed: pattern_speed(patterns.blinky_elroy_one),
        elroy_two_dots_left,
        elroy_two_speed: pattern_speed(patterns.blinky_elroy_two),
        frightened_ghost_speed: frightened_available
            .then_some(pattern_speed(patterns.ghost_frightened)),
        frightened_time,
    }
}

pub fn scatter_durations(level: u32) -> [f32; 4] {
    let frames = phase_change_frames(level);
    [
        frames_to_seconds(frames[0]),
        frames_to_seconds(frames[2] - frames[1]),
        frames_to_seconds(frames[4] - frames[3]),
        frames_to_seconds(frames[6] - frames[5]),
    ]
}

pub fn chase_durations(level: u32) -> [Option<f32>; 4] {
    let frames = phase_change_frames(level);
    [
        Some(frames_to_seconds(frames[1] - frames[0])),
        Some(frames_to_seconds(frames[3] - frames[2])),
        Some(frames_to_seconds(frames[5] - frames[4])),
        None,
    ]
}

pub fn ghost_personal_dot_limit(kind: GhostKind, level: u32) -> usize {
    let group_index = difficulty_entry(level)[2] as usize;
    let group = arcade_rule_tables().personal_dot_groups[group_index];
    match kind {
        GhostKind::Pinky => group[0] as usize,
        GhostKind::Inky => group[1] as usize,
        GhostKind::Clyde => group[2] as usize,
        GhostKind::Blinky => 0,
    }
}

pub fn global_release_dot(kind: GhostKind) -> Option<usize> {
    match kind {
        GhostKind::Pinky => Some(usize::from(arcade_rule_tables().global_release_dots[0])),
        GhostKind::Inky => Some(usize::from(arcade_rule_tables().global_release_dots[1])),
        GhostKind::Clyde => Some(usize::from(arcade_rule_tables().global_release_dots[2])),
        GhostKind::Blinky => None,
    }
}

pub fn release_timer_limit(level: u32) -> f32 {
    let frame_index = difficulty_entry(level)[5] as usize;
    f32::from(arcade_rule_tables().release_timer_frames[frame_index]) / ORIGINAL_FPS
}

pub fn fright_flash_duration(total_time: f32) -> f32 {
    total_time.min((f32::from(FRIGHT_FLASH_START_TICKS) - 1.0) / ARCADE_TIMER_TICKS_PER_SECOND)
}

pub fn fright_flash_half_period_seconds() -> f32 {
    f32::from(FRIGHT_FLASH_HALF_PERIOD_TICKS) / ARCADE_TIMER_TICKS_PER_SECOND
}

pub fn fruit_release_dots() -> [usize; 2] {
    arcade_rule_tables().fruit_release_dots.map(usize::from)
}

pub fn fruit_lifespan_seconds() -> f32 {
    isr_delay_seconds(arcade_rule_tables().fruit_reset_timer)
}

pub fn move_patterns(level: u32) -> ArcadeMovePatterns {
    let group = arcade_rule_tables().move_pattern_groups[difficulty_entry(level)[0] as usize];
    ArcadeMovePatterns {
        pacman_normal: group[0],
        pacman_frightened: group[1],
        blinky_elroy_two: group[2],
        blinky_elroy_one: group[3],
        ghost_normal: group[4],
        ghost_frightened: group[5],
        ghost_tunnel: group[6],
    }
}

pub fn dot_pause_seconds(power_pellet: bool) -> f32 {
    ORIGINAL_FPS.recip() * if power_pellet { 3.0 } else { 1.0 }
}

fn arcade_rule_tables() -> &'static ArcadeRuleTables {
    static TABLES: OnceLock<ArcadeRuleTables> = OnceLock::new();
    TABLES.get_or_init(|| parse_arcade_rule_tables(ARCADE_RULES))
}

fn difficulty_entry(level: u32) -> [u8; 6] {
    arcade_rule_tables().difficulty_entries[level_index(level)]
}

fn phase_change_frames(level: u32) -> [u16; 7] {
    let group_index = difficulty_entry(level)[0] as usize;
    arcade_rule_tables().phase_change_frames[group_index]
}

fn elroy_dot_limits(level: u32) -> (usize, usize) {
    let group_index = difficulty_entry(level)[3] as usize;
    let [first, second] = arcade_rule_tables().elroy_dot_groups[group_index];
    (usize::from(first), usize::from(second))
}

fn fruit_points(level: u32) -> u32 {
    arcade_rule_tables().fruit_points[level_index(level)]
}

fn frightened_time_seconds(level: u32) -> f32 {
    let timer_index = difficulty_entry(level)[4] as usize;
    f32::from(arcade_rule_tables().frightened_timer_ticks[timer_index])
        / ARCADE_TIMER_TICKS_PER_SECOND
}

fn frames_to_seconds(frames: u16) -> f32 {
    f32::from(frames) / ORIGINAL_FPS
}

fn pattern_speed(pattern: u32) -> f32 {
    pattern.count_ones() as f32 / 20.0
}

fn isr_delay_seconds(encoded: u8) -> f32 {
    let unit = match encoded >> 6 {
        0 => ORIGINAL_FPS.recip(),
        1 => 0.1,
        2 => 1.0,
        _ => 10.0,
    };
    unit * f32::from(encoded & 0x3f)
}

fn level_index(level: u32) -> usize {
    level.saturating_sub(1).min(20) as usize
}

fn parse_arcade_rule_tables(text: &str) -> ArcadeRuleTables {
    let mut difficulty_entries = None;
    let mut personal_dot_groups = None;
    let mut elroy_dot_groups = None;
    let mut frightened_timer_ticks = None;
    let mut release_timer_frames = None;
    let mut move_pattern_groups = None;
    let mut phase_change_frames = None;
    let mut fruit_points = None;
    let mut fruit_reset_timer = None;
    let mut fruit_release_dots = None;
    let mut global_release_dots = None;

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let (key, value) = line
            .split_once('=')
            .expect("arcade rule metadata lines should use key=value");
        match key {
            "difficulty_entries" => difficulty_entries = Some(parse_table_6x21(value)),
            "personal_dot_groups" => personal_dot_groups = Some(parse_table_3x4(value)),
            "elroy_dot_groups" => elroy_dot_groups = Some(parse_table_2x8(value)),
            "frightened_timer_ticks" => {
                frightened_timer_ticks = Some(parse_frightened_timer_ticks(value))
            }
            "release_timer_frames" => release_timer_frames = Some(parse_release_frames(value)),
            "move_pattern_groups" => move_pattern_groups = Some(parse_hex_table_7x7(value)),
            "phase_change_frames" => phase_change_frames = Some(parse_table_7x7(value)),
            "fruit_points" => fruit_points = Some(parse_fruit_points(value)),
            "fruit_reset_timer" => {
                fruit_reset_timer = Some(value.parse().expect("fruit reset timer should parse"))
            }
            "fruit_release_dots" => fruit_release_dots = Some(parse_release_dots(value)),
            "global_release_dots" => global_release_dots = Some(parse_global_release_dots(value)),
            _ => {}
        }
    }

    ArcadeRuleTables {
        difficulty_entries: difficulty_entries
            .expect("arcade rule metadata should define difficulty_entries"),
        personal_dot_groups: personal_dot_groups
            .expect("arcade rule metadata should define personal_dot_groups"),
        elroy_dot_groups: elroy_dot_groups
            .expect("arcade rule metadata should define elroy_dot_groups"),
        frightened_timer_ticks: frightened_timer_ticks
            .expect("arcade rule metadata should define frightened_timer_ticks"),
        release_timer_frames: release_timer_frames
            .expect("arcade rule metadata should define release_timer_frames"),
        move_pattern_groups: move_pattern_groups
            .expect("arcade rule metadata should define move_pattern_groups"),
        phase_change_frames: phase_change_frames
            .expect("arcade rule metadata should define phase_change_frames"),
        fruit_points: fruit_points.expect("arcade rule metadata should define fruit_points"),
        fruit_reset_timer: fruit_reset_timer
            .expect("arcade rule metadata should define fruit_reset_timer"),
        fruit_release_dots: fruit_release_dots
            .expect("arcade rule metadata should define fruit_release_dots"),
        global_release_dots: global_release_dots
            .expect("arcade rule metadata should define global_release_dots"),
    }
}

fn parse_table_6x21(value: &str) -> [[u8; 6]; 21] {
    parse_table::<6, 21>(value)
}

fn parse_table_3x4(value: &str) -> [[u8; 3]; 4] {
    parse_table::<3, 4>(value)
}

fn parse_table_2x8(value: &str) -> [[u8; 2]; 8] {
    parse_table::<2, 8>(value)
}

fn parse_table_7x7(value: &str) -> [[u16; 7]; 7] {
    parse_u16_table::<7, 7>(value)
}

fn parse_hex_table_7x7(value: &str) -> [[u32; 7]; 7] {
    value
        .split(';')
        .map(|row| {
            row.split(',')
                .map(|item| {
                    let hex = item
                        .strip_prefix("0x")
                        .or_else(|| item.strip_prefix("0X"))
                        .unwrap_or(item);
                    u32::from_str_radix(hex, 16).expect("hex table value should parse")
                })
                .collect::<Vec<_>>()
                .try_into()
                .expect("table row should have fixed width")
        })
        .collect::<Vec<[u32; 7]>>()
        .try_into()
        .expect("table should have fixed row count")
}

fn parse_frightened_timer_ticks(value: &str) -> [u16; 9] {
    value
        .split(',')
        .map(|item| {
            item.parse::<u16>()
                .expect("frightened timer tick should parse")
        })
        .collect::<Vec<_>>()
        .try_into()
        .expect("frightened timer table should have nine values")
}

fn parse_fruit_points(value: &str) -> [u32; 21] {
    value
        .split(',')
        .map(|item| item.parse::<u32>().expect("fruit points should parse"))
        .collect::<Vec<_>>()
        .try_into()
        .expect("fruit point table should have twenty-one values")
}

fn parse_release_dots(value: &str) -> [u8; 2] {
    value
        .split(',')
        .map(|item| item.parse::<u8>().expect("fruit release dot should parse"))
        .collect::<Vec<_>>()
        .try_into()
        .expect("fruit release table should have two values")
}

fn parse_global_release_dots(value: &str) -> [u8; 3] {
    value
        .split(',')
        .map(|item| item.parse::<u8>().expect("global release dot should parse"))
        .collect::<Vec<_>>()
        .try_into()
        .expect("global release table should have three values")
}

fn parse_table<const WIDTH: usize, const ROWS: usize>(value: &str) -> [[u8; WIDTH]; ROWS] {
    value
        .split(';')
        .map(|row| {
            row.split(',')
                .map(|item| item.parse::<u8>().expect("table value should parse"))
                .collect::<Vec<_>>()
                .try_into()
                .expect("table row should have fixed width")
        })
        .collect::<Vec<[u8; WIDTH]>>()
        .try_into()
        .expect("table should have fixed row count")
}

fn parse_u16_table<const WIDTH: usize, const ROWS: usize>(value: &str) -> [[u16; WIDTH]; ROWS] {
    value
        .split(';')
        .map(|row| {
            row.split(',')
                .map(|item| item.parse::<u16>().expect("table value should parse"))
                .collect::<Vec<_>>()
                .try_into()
                .expect("table row should have fixed width")
        })
        .collect::<Vec<[u16; WIDTH]>>()
        .try_into()
        .expect("table should have fixed row count")
}

fn parse_release_frames(value: &str) -> [u16; 3] {
    value
        .split(',')
        .map(|item| item.parse::<u16>().expect("frame count should parse"))
        .collect::<Vec<_>>()
        .try_into()
        .expect("release timer table should have three values")
}

#[cfg(test)]
mod tests {
    use super::{
        MovePatternState, ORIGINAL_FPS, chase_durations, fright_flash_duration,
        fruit_lifespan_seconds, fruit_release_dots, ghost_personal_dot_limit, level_spec,
        move_patterns, release_timer_limit, scatter_durations,
    };
    use crate::actors::GhostKind;

    #[test]
    fn level_one_matches_arcade_basics() {
        let spec = level_spec(1);
        assert_eq!(spec.fruit_points, 100);
        assert_eq!(spec.elroy_one_dots_left, 20);
        assert_eq!(spec.frightened_time, 6.0);
    }

    #[test]
    fn release_rules_match_arcade_thresholds() {
        assert_eq!(ghost_personal_dot_limit(GhostKind::Pinky, 1), 0);
        assert_eq!(ghost_personal_dot_limit(GhostKind::Inky, 1), 30);
        assert_eq!(ghost_personal_dot_limit(GhostKind::Clyde, 2), 50);
        assert_eq!(super::global_release_dot(GhostKind::Pinky), Some(7));
        assert_eq!(super::global_release_dot(GhostKind::Inky), Some(17));
        assert_eq!(super::global_release_dot(GhostKind::Clyde), Some(32));
        assert!((release_timer_limit(4) - (240.0 / 60.606_06)).abs() < 0.001);
        assert!((release_timer_limit(5) - (180.0 / 60.606_06)).abs() < 0.001);
        assert_eq!(fruit_release_dots(), [70, 170]);
        assert!((fruit_lifespan_seconds() - 10.0).abs() < 0.001);
    }

    #[test]
    fn frightened_timers_follow_the_rom_table() {
        assert!((level_spec(1).frightened_time - 6.0).abs() < 0.001);
        assert!((level_spec(14).frightened_time - 3.0).abs() < 0.001);
        assert!((level_spec(17).frightened_time - (1.0 / 120.0)).abs() < 0.001);
        assert!((fright_flash_duration(6.0) - (255.0 / 120.0)).abs() < 0.001);
        assert!((fright_flash_duration(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn scatter_and_chase_durations_follow_rom_phase_thresholds() {
        let level_one_scatters = scatter_durations(1);
        let level_one_chases = chase_durations(1);
        let level_two_scatters = scatter_durations(2);

        assert!((level_one_scatters[0] - (420.0 / ORIGINAL_FPS)).abs() < 0.001);
        assert!((level_one_scatters[3] - (300.0 / ORIGINAL_FPS)).abs() < 0.001);
        assert!(
            (level_one_chases[0].expect("level one chase should exist") - (1200.0 / ORIGINAL_FPS))
                .abs()
                < 0.001
        );
        assert!((level_two_scatters[3] - ORIGINAL_FPS.recip()).abs() < 0.001);
        assert_eq!(chase_durations(2)[3], None);
    }

    #[test]
    fn move_patterns_follow_the_rom_move_groups() {
        let level_one = move_patterns(1);
        let level_two = move_patterns(2);

        assert_eq!(level_one.pacman_normal, 0x55555555);
        assert_eq!(level_one.pacman_frightened, 0xd56ad56a);
        assert_eq!(level_one.ghost_normal, 0xaa2a5555);
        assert_eq!(level_one.ghost_tunnel, 0x22222222);
        assert_eq!(level_two.ghost_tunnel, 0x48242291);
        assert_eq!(level_two.blinky_elroy_two, 0xd65aadb5);
    }

    #[test]
    fn move_pattern_state_uses_the_rom_bit_cycle() {
        let mut state = MovePatternState::new(0xa000_0001);

        let first_cycle: Vec<_> = (0..4).map(|_| state.advance()).collect();
        assert_eq!(first_cycle, vec![true, false, true, false]);

        state.reset();
        let moves = (0..32).filter(|_| state.advance()).count();
        assert_eq!(moves, state.base().count_ones() as usize);
    }
}
