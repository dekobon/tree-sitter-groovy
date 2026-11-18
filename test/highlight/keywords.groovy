class Foo { }
// <- keyword

trait T { }
// <- keyword

interface I { }
// <- keyword

enum E { A }
// <- keyword

record R(int x)
// <- keyword

package com.example
// <- keyword

import java.util.List
// <- keyword

def x = 5
// <- keyword

var y = 6
// <- keyword

sealed class S { }
// <- keyword

non-sealed class N permits A { }
// <- keyword

sealed class P permits B { }
//               ^ keyword

pipeline { }
// <- keyword

if (x) y
// <- keyword.control

while (x) y
// <- keyword.control

for (i in xs) y
// <- keyword.control

return x
// <- keyword.control

throw e
// <- keyword.control

try { x }
// <- keyword.control

switch (x) { case 1 -> y }
// <- keyword.control

switch (x) { default -> y }
//           ^ keyword.control

@interface Tag { String value() default 'x' }
//                              ^ keyword

interface Greeter {
    default String greet() { 'hi' }
    // <- keyword.modifier
    public default int pick(int x) { switch (x) { default -> 0 } }
    //     ^ keyword.modifier
    //                                            ^ keyword.control
}

default class Plain { }
// <- keyword.modifier
default interface Face { }
// <- keyword.modifier
default trait Mixin { }
// <- keyword.modifier
default enum Level { LOW }
// <- keyword.modifier
default record Pair(int a) { }
// <- keyword.modifier

default void script() { }
// <- keyword.modifier

foo default: 1
//  ^ label

yield 42
// <- keyword.control

record Q(int x) { }
// <- keyword

record.key()
// <- variable

record(1)
// <- function.call

class Holder {
    private Record record
    //             ^ property
    record Inner(int x) { }
    // <- keyword
}

class Hero {
    private String trait
    //             ^ property
    private boolean sealed = true
    //              ^ property
    trait Skill { }
    // <- keyword
    sealed class Rank { }
    // <- keyword
}

trait = sealed
// <- variable
//      ^ variable

synchronized (lock) { }
// <- keyword

class Sync {
    synchronized void run() { }
    // <- keyword.modifier
}

xs.each { a, b -> a }
//             ^ operator

def v(String... rest) { }
//          ^ operator

def f(def a, var b) { }
//    ^ keyword
//           ^ keyword

final var (int a, String b) = pair()
// <- keyword.modifier
//    ^ keyword
//         ^ type.builtin
//             ^ variable
//                ^ type
//                       ^ variable

class Pair {
    static final def (left, right) = [1, 2]
    //                ^ variable
    //                      ^ variable
}
