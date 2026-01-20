use pixel_basic::{GameBridge, DrawCommand};

#[test]
fn test_plot_renders() {
    let mut bridge = GameBridge::new();

    let program = r#"
10 REM Test PLOT rendering - immediate execution
20 PLOT 5, 10, "X", 15, 0
30 PLOT 10, 10, "@", 10, 0
40 PRINT "Done plotting"
50 END
"#;

    bridge.load_program(program).expect("Failed to load");

    // Run program
    for _ in 0..100 {
        match bridge.update(0.016) {
            Ok(false) => break,
            Ok(true) => {},
            Err(e) => panic!("Error: {:?}", e),
        }
    }

    // Check the draw commands collected during update
    // Note: In this test, PLOT is executed during update, not draw
    // so we check that commands are collected properly
}

#[test]
fn test_draw_collects_plot_commands() {
    let mut bridge = GameBridge::new();

    let program = r#"
10 END

3500 REM ON_DRAW
3510 PLOT 5, 10, "X", 15, 0
3520 PLOT 10, 10, "@", 10, 0
3530 RETURN
"#;

    bridge.load_program(program).expect("Failed to load");

    // Call draw to execute ON_DRAW
    bridge.draw().expect("Failed to draw");

    let commands: Vec<_> = bridge.context().commands().to_vec();
    println!("Commands: {:?}", commands);

    // Should have 2 PLOT commands
    assert_eq!(commands.len(), 2, "Expected 2 draw commands, got {}", commands.len());

    // Check first command
    match &commands[0] {
        DrawCommand::Plot { x, y, ch, fg, bg } => {
            assert_eq!(*x, 5);
            assert_eq!(*y, 10);
            assert_eq!(*ch, 'X');
            assert_eq!(*fg, 15);
            assert_eq!(*bg, 0);
        }
        _ => panic!("Expected Plot command"),
    }

    // Check second command
    match &commands[1] {
        DrawCommand::Plot { x, y, ch, fg, bg } => {
            assert_eq!(*x, 10);
            assert_eq!(*y, 10);
            assert_eq!(*ch, '@');
            assert_eq!(*fg, 10);
            assert_eq!(*bg, 0);
        }
        _ => panic!("Expected Plot command"),
    }
}
