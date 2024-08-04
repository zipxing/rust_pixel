#!/usr/bin/env python
# coding=utf-8
from coloraide import Color 
from coloraide.spaces.cam16_jmh import CAM16JMh, Environment
from coloraide.cat import WHITES
from coloraide import util
import math
Color.register([CAM16JMh()])
print(Color('white').convert('cam16-jmh'))
print(Color('white').convert('xyz-d65'))
