def classify(x) {
    if (x < 0) {
        return 'negative'
    } else if (x == 0) {
        return 'zero'
    } else {
        return 'positive'
    }
}

def sumUp(xs) {
    def total = 0
    for (n in xs) {
        total = total + n
    }
    return total
}

def countDown(n) {
    while (n > 0) {
        n = n - 1
    }
    return n
}

def describe(c) {
    switch (c) {
        case 1 -> 'one'
        case 2 -> 'two'
        case 3 -> 'three'
        default -> 'many'
    }
}

def safeRun(action) {
    try {
        action()
    } catch (IllegalStateException | IllegalArgumentException e) {
        e.message
    } catch (Throwable t) {
        'unknown'
    } finally {
        action()
    }
}

def withResource() {
    try (def r = open()) {
        r.use()
    }
}

outer: for (i in 0..10) {
    inner: for (j in 0..10) {
        if (i + j > 15) {
            break outer
        }
    }
}
