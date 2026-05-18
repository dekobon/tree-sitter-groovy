def a = 1 + 2 * 3
def b = (a + 1) * (a - 1)
def c = 2 ** 8
def d = 0xFFFFL
def e = 1_000_000.0
def f = 1.5e-7

def lo = 0
def hi = 10
def inclusive = lo..hi
def exclusive_right = lo..<hi
def exclusive_left = lo<..hi
def exclusive_both = lo<..<hi

def sum = a + b + c + d + e + f
def diff = a - b
def power = (a + 1) ** 2

def shifted = a << 2
def masked = 0xFFFF & 0x00FF
