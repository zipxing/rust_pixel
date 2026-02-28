use pixel_basic::GameBridge;

#[test]
fn test_snake_array_initialization() {
    let mut bridge = GameBridge::new();

    // Simplified version of game.bas initialization
    let program = r#"
10 REM Test array initialization like in snake game
1000 REM ON_INIT
1010 X = 20: Y = 10
1020 L = 5
1030 DIM BX(100): DIM BY(100)
1040 FOR I = 0 TO L - 1
1050   BX(I) = X - I: BY(I) = Y
1060 NEXT I
1070 PRINT "INIT DONE"
1080 RETURN
2000 REM ON_TICK
2010 RETURN
3500 REM ON_DRAW
3510 FOR I = 1 TO L - 1
3520   PLOT BX(I), BY(I), "O", 2, 0
3530 NEXT I
3540 PRINT "DRAW DONE"
3550 RETURN
9000 END
"#;

    bridge.load_program(program).expect("Failed to load program");

    // Call ON_INIT
    println!("Calling ON_INIT...");
    bridge.call_subroutine(1000).expect("Failed to call ON_INIT");

    // Call ON_DRAW
    println!("Calling ON_DRAW...");
    match bridge.call_subroutine(3500) {
        Ok(_) => println!("ON_DRAW completed successfully"),
        Err(e) => panic!("ON_DRAW failed: {:?}", e),
    }
}
