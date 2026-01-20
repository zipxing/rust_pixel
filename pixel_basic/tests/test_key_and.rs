use pixel_basic::{GameBridge, NullGameContext};

#[test]
fn test_key_and_expression() {
    let mut bridge = GameBridge::new(NullGameContext);

    // Test KEY() function with AND operator
    let program = r#"
10 REM Test KEY() with AND operator
20 DY = 0
30 IF KEY("W") AND DY = 0 THEN PRINT "W pressed and DY is 0"
40 END
"#;

    println!("Loading program...");
    bridge.load_program(program).expect("Failed to load program");
    println!("Program loaded successfully");

    // Execute the program
    for _ in 0..10 {
        match bridge.update(0.016) {
            Ok(true) => {},  // Continue
            Ok(false) => {
                println!("Program completed successfully");
                break;
            },
            Err(e) => {
                panic!("Error during execution: {:?}", e);
            }
        }
    }
}
