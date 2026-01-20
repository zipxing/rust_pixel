10 REM ============================================
20 REM SIMPLE TEST - Basic functionality test
30 REM ============================================
40 GOSUB 1000
50 YIELD
60 GOTO 50
1000 REM ON_INIT
1010 PRINT "Initializing..."
1020 X = 20
1030 Y = 10
1040 RETURN
2000 REM ON_TICK
2010 X = X + 1
2020 IF X > 40 THEN X = 20
2030 RETURN
3500 REM ON_DRAW
3510 REM Just test if draw is called
3520 RETURN
