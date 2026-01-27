10 REM ============================================
20 REM SNAKE GAME - Written in BASIC for rust_pixel
30 REM ============================================
40 REM Main loop - GameBridge automatically calls hooks
50 REM GameBridge will call: ON_INIT(1000), ON_TICK(2000), ON_DRAW(3500)
60 YIELD: REM Let GameBridge manage the game loop
70 GOTO 60
80 END
90 REM

100 REM ============================================
1000 REM ON_INIT - Game initialization (called once at start)
1010 REM ============================================
1020 REM Initialize snake body arrays (only once!)
1030 DIM BX(100): DIM BY(100)
1040 REM Call reset to set initial game state
1050 GOSUB 1500
1060 RETURN

1500 REM RESET_GAME - Reset game state (called on init and restart)
1510 REM ============================================
1520 CLS
1530 REM Initialize snake position and direction
1540 X = 20: Y = 10
1550 DX = 1: DY = 0
1560 L = 5: REM Initial snake length
1570 REM Initialize food position
1580 FX = 30: FY = 15
1590 SC = 0: REM Score
1595 DEAD = 0: REM Game over flag (0=alive, 1=dead)
1596 FC = 0: REM Frame counter for speed control
1600 REM Reset snake body positions
1610 FOR I = 0 TO L - 1
1620   BX(I) = X - I: BY(I) = Y
1630 NEXT I
1640 REM Draw initial border
1650 BOX 0, 0, 60, 24, 1
1660 REM Display initial score
1670 PRINT "SCORE: "; SC
1680 RETURN

2000 REM ============================================
2010 REM ON_TICK - Game logic (called every frame)
2020 REM ============================================
2025 REM If dead, check for restart
2026 IF DEAD = 1 THEN GOTO 2500
2030 REM Handle input (can change direction every frame)
2040 IF KEY("W") THEN DX = 0: DY = -1
2050 IF KEY("S") THEN DX = 0: DY = 1
2060 IF KEY("A") THEN DX = -1: DY = 0
2070 IF KEY("D") THEN DX = 1: DY = 0
2075 REM
2076 REM Speed control: only move every 6 frames (10 moves per second at 60fps)
2077 FC = FC + 1
2078 IF FC < 6 THEN RETURN
2079 FC = 0
2080 REM
2090 REM Move snake body (shift all segments)
2100 FOR I = L - 1 TO 1 STEP -1
2110   BX(I) = BX(I - 1)
2120   BY(I) = BY(I - 1)
2130 NEXT I
2140 REM
2150 REM Move head
2160 X = X + DX: Y = Y + DY
2170 BX(0) = X: BY(0) = Y
2180 REM
2190 REM Wrap around walls (穿墙)
2200 IF X <= 0 THEN X = 58
2201 IF X >= 59 THEN X = 1
2202 IF Y <= 0 THEN Y = 22
2203 IF Y >= 23 THEN Y = 1
2204 BX(0) = X: BY(0) = Y
2210 REM
2220 REM Check self collision
2230 FOR I = 1 TO L - 1
2240   IF X = BX(I) AND Y = BY(I) THEN DEAD = 1: RETURN
2250 NEXT I
2260 REM
2270 REM Check food collision
2280 IF X = FX AND Y = FY THEN GOSUB 3000: REM Eat food
2290 REM
2300 RETURN
2500 REM Dead state - wait for restart
2510 IF KEY("SPACE") THEN GOSUB 1500
2520 RETURN
3000 REM ============================================
3010 REM EAT_FOOD - Handle food eating
3020 REM ============================================
3030 L = L + 1: REM Grow snake
3040 SC = SC + 10: REM Increase score
3050 REM Generate new food position
3060 FX = INT(RND(0) * 57) + 1
3070 FY = INT(RND(0) * 21) + 1
3080 REM Make sure food is not on snake
3090 FOR I = 0 TO L - 1
3100   IF FX = BX(I) AND FY = BY(I) THEN GOTO 3060: REM Regenerate
3110 NEXT I
3120 RETURN
3500 REM ============================================
3510 REM ON_DRAW - Rendering (called every frame)
3520 REM ============================================
3530 REM Clear play area (not border)
3540 FOR YY = 1 TO 22
3550   FOR XX = 1 TO 58
3560     PLOT XX, YY, " ", 7, 0
3570   NEXT XX
3580 NEXT YY
3590 REM
3600 REM Draw snake body
3610 FOR I = 1 TO L - 1
3620   PLOT BX(I), BY(I), "O", 2, 0
3630 NEXT I
3640 REM Draw snake head
3650 PLOT X, Y, "@", 10, 0
3660 REM
3670 REM Draw food
3680 PLOT FX, FY, "*", 14, 0
3690 REM
3700 REM Update score display
3710 PLOT 1, 0, " ", 7, 0: REM Clear old score
3720 PLOT 2, 0, " ", 7, 0
3730 PLOT 3, 0, " ", 7, 0
3740 PLOT 4, 0, " ", 7, 0
3750 PLOT 5, 0, " ", 7, 0
3760 PLOT 6, 0, " ", 7, 0
3770 PLOT 7, 0, "S", 15, 0
3780 PLOT 8, 0, "C", 15, 0
3790 PLOT 9, 0, "O", 15, 0
3800 PLOT 10, 0, "R", 15, 0
3810 PLOT 11, 0, "E", 15, 0
3820 PLOT 12, 0, ":", 15, 0
3830 REM Display score (simple digit display)
3840 S$ = STR$(SC)
3850 FOR I = 1 TO LEN(S$)
3860   C$ = MID$(S$, I, 1)
3870   PLOT 12 + I, 0, C$, 11, 0
3880 NEXT I
3890 REM
3895 REM Show game over message if dead
3896 IF DEAD = 1 THEN GOSUB 4000
3900 RETURN
4000 REM ============================================
4010 REM GAME_OVER - Display game over message
4020 REM ============================================
4030 REM Display game over message
4040 PLOT 25, 12, "G", 12, 0
4050 PLOT 26, 12, "A", 12, 0
4060 PLOT 27, 12, "M", 12, 0
4070 PLOT 28, 12, "E", 12, 0
4080 PLOT 29, 12, " ", 12, 0
4090 PLOT 30, 12, "O", 12, 0
4100 PLOT 31, 12, "V", 12, 0
4110 PLOT 32, 12, "E", 12, 0
4120 PLOT 33, 12, "R", 12, 0
4130 REM Show restart hint
4140 PLOT 22, 14, "P", 7, 0
4150 PLOT 23, 14, "R", 7, 0
4160 PLOT 24, 14, "E", 7, 0
4170 PLOT 25, 14, "S", 7, 0
4180 PLOT 26, 14, "S", 7, 0
4190 PLOT 27, 14, " ", 7, 0
4200 PLOT 28, 14, "S", 7, 0
4210 PLOT 29, 14, "P", 7, 0
4220 PLOT 30, 14, "A", 7, 0
4230 PLOT 31, 14, "C", 7, 0
4240 PLOT 32, 14, "E", 7, 0
4250 RETURN
