//! The binary's stdout must equal what `steel-current` prints for the same scenario.
//!
//! `fixtures/steel_current_main.txt` was produced by running `steel-current` unmodified:
//! `cargo run -q -p steel-current > steel-ecs-bevy/tests/fixtures/steel_current_main.txt`

use std::process::Command;

#[test]
fn main_output_matches_steel_current_byte_for_byte() {
    let out = Command::new(env!("CARGO_BIN_EXE_steel-ecs-bevy"))
        .output()
        .expect("run binary");
    assert!(out.status.success());
    let actual = String::from_utf8(out.stdout).unwrap();
    let expected = include_str!("fixtures/steel_current_main.txt");
    assert_eq!(actual, expected);
}
