use pixel_basic::GameBridge;

#[test]
fn test_snake_game_lifecycle() {
    let mut bridge = GameBridge::new();

    // Minimal version of game.bas that reproduces the issue
    let program = r#"
10 REM Main loop
60 YIELD
70 GOTO 60
80 END

1000 REM ON_INIT
1010 X = 20: Y = 10
1020 L = 5
1030 DIM BX(100): DIM BY(100)
1040 FOR I = 0 TO L - 1
1050   BX(I) = X - I: BY(I) = Y
1060 NEXT I
1070 PRINT "INIT DONE, L="; L
1080 RETURN

2000 REM ON_TICK
2010 REM Move snake body
2020 FOR I = L - 1 TO 1 STEP -1
2030   BX(I) = BX(I - 1)
2040   BY(I) = BY(I - 1)
2050 NEXT I
2060 REM Move head
2070 X = X + 1: Y = Y
2080 BX(0) = X: BY(0) = Y
2090 RETURN

3500 REM ON_DRAW
3510 FOR I = 1 TO L - 1
3520   PLOT BX(I), BY(I), "O", 2, 0
3530 NEXT I
3540 PLOT X, Y, "@", 10, 0
3550 REM Score display
3560 SC = 100
3570 S$ = STR$(SC)
3580 FOR I = 1 TO LEN(S$)
3590   C$ = MID$(S$, I, 1)
3600   PLOT 12 + I, 0, C$, 11, 0
3610 NEXT I
3620 RETURN
"#;

    bridge.load_program(program).expect("Failed to load program");

    // Simulate game loop: update() then draw()
    for frame in 1..=100 {
        println!("\n=== Frame {} ===", frame);

        // update() calls ON_INIT (first time) and ON_TICK
        match bridge.update(0.016) {
            Ok(_) => println!("update() OK"),
            Err(e) => {
                panic!("Frame {}: Error in update(): {:?}", frame, e);
            }
        }

        // draw() calls ON_DRAW
        match bridge.call_subroutine(3500) {
            Ok(_) => println!("draw() OK"),
            Err(e) => {
                panic!("Frame {}: Error in draw(): {:?}", frame, e);
            }
        }
    }

    println!("\nTest completed successfully!");
}
