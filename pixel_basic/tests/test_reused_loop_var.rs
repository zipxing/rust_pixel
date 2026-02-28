use pixel_basic::GameBridge;

#[test]
fn test_reused_loop_variable() {
    let mut bridge = GameBridge::new();

    // Test if loop variable I can be reused across subroutines
    let program = r#"
10 REM Test reused loop variable
20 DIM A(10)
30 FOR I = 0 TO 5
40   A(I) = I * 10
50 NEXT I
60 GOSUB 1000
70 GOSUB 2000
80 END

1000 REM Subroutine 1 - uses I in a loop
1010 FOR I = 0 TO 3
1020   PRINT "Sub1: I="; I; " A(I)="; A(I)
1030 NEXT I
1040 RETURN

2000 REM Subroutine 2 - uses I in a different context
2010 S$ = STR$(42)
2020 FOR I = 1 TO LEN(S$)
2030   C$ = MID$(S$, I, 1)
2040   PRINT "Sub2: I="; I; " C$="; C$
2050 NEXT I
2060 REM Now try to use array with I
2070 FOR I = 0 TO 2
2080   PRINT "Sub2 array: I="; I; " A(I)="; A(I)
2090 NEXT I
2100 RETURN
"#;

    bridge.load_program(program).expect("Failed to load program");

    // Execute the program
    for _ in 0..1000 {
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
