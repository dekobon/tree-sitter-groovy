def a = 5
def b = 3

def cmp = a <=> b
def identity = a === b
def diff = a !== b

def isOne = a == 1
def isLess = a < b
def isOdd = a % 2 == 1

def picked = a ?: b
def ternary = a > b ? a : b
def assertion = a >= 0
def implication = a > 0 ==> b > 0

def matched = "hello" =~ "[a-z]+"
def fullmatch = "abc" ==~ "^a.c$"

def safe = a?.toString()
def safeChain = a??.toString()
def methodPtr = a.&toString
def directField = a.@value
def methodRef = String::length
def spreadProp = [1, 2, 3]*.toString()

def updated = a++
def prefixed = ++b

def assigned = a
assigned = b
assigned += 1
assigned -= 1
assigned *= 2
assigned /= 2
assigned %= 3
assigned **= 2
assigned <<= 1
assigned >>= 1
assigned >>>= 1
assigned &= 0xFF
assigned ^= 0x0F
assigned |= 0xF0
assigned ?= b
