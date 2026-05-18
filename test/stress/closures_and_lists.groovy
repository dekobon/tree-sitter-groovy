def numbers = [1, 2, 3, 4, 5]
def doubled = numbers.collect({ n -> n * 2 })
def filtered = numbers.findAll({ n -> n > 2 })

def lookup = [
    one: 1,
    two: 2,
    three: 3,
]

def composed = [
    *: lookup,
    four: 4,
]

def nested = [
    [1, 2, 3],
    [4, 5, 6],
    [7, 8, 9],
]

def operations = [
    add: { a, b -> a + b },
    sub: { a, b -> a - b },
    mul: { a, b -> a * b },
]

def callOp(map, op, a, b) {
    map[op](a, b)
}

def spread_call(xs) {
    operations.add(*xs)
}

println 'starting'
print numbers
debug { numbers.size() }
log 'done'
