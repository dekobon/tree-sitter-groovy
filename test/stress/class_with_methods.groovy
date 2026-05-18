package com.example

class Point {
    def distance(Point other) {
        def dx = x - other.x
        def dy = y - other.y
        Math.sqrt(dx * dx + dy * dy)
    }

    def translated(int delta = 1) {
        new Point(x + delta, y + delta)
    }

    def equals(Object other) {
        if (other instanceof Point) {
            x == other.x && y == other.y
        } else {
            false
        }
    }
}

trait Named {
    def name() {
        "unknown"
    }
}

interface Shape {
    def area()
    def perimeter()
}

@interface Stable {
    def since()
}
