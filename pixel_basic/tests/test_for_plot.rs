use pixel_basic::GameBridge;

#[test]
fn test_for_loop_with_plot() {
    let mut bridge = GameBridge::new();

    let program = r#"
10 REM Test FOR loop with PLOT
20 FOR I = 1 TO 5
30   PLOT I, 10, "X", 15, 0
40 NEXT I
50 END
"#;

    bridge.load_program(program).expect("Failed to load program");

    // Execute the program using update() which calls step internally
    for _ in 0..100 {
        match bridge.update(0.016) {
            Ok(true) => {},  // Continue
            Ok(false) => break,  // Program ended
            Err(e) => {
                panic!("Error during execution: {:?}", e);
            }
        }
    }

    // If we get here without panicking, the test passed
    println!("Test passed: FOR loop with PLOT executed without errors");
}
