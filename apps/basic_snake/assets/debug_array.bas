10 REM Debug array access
20 PRINT "Test 1: Create array"
30 DIM A(10)
40 PRINT "Test 2: Set values"
50 A(0) = 5
60 A(1) = 10
70 PRINT "Test 3: Read values"
80 PRINT "A(0) = "; A(0)
90 PRINT "A(1) = "; A(1)
100 PRINT "Test 4: Use in expression"
110 X = A(0) + A(1)
120 PRINT "X = "; X
130 PRINT "Test 5: Use in PLOT (simulated)"
140 Y = A(0)
150 Z = A(1)
160 PRINT "Y = "; Y; " Z = "; Z
170 END
