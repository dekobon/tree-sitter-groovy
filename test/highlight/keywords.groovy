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

class F { var f = 1; var String g }
//        ^ keyword
//                   ^ keyword
//                       ^ type

class G { var m() { } }
//        ^ type

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

assert x == 1, 'x must be one'
// <- keyword.control
//             ^ string

// A label is on the line of `break` or `continue`; a name on the next line
// is the next statement.
while (x) {
    break outer
    //    ^ label
    continue
    outer
//  ^ variable
//  ^ !label
}

try { x }
// <- keyword.control

switch (x) { case 1 -> y }
// <- keyword.control

switch (x) { default -> y }
//           ^ keyword.control

switch (x) { case 1, y -> z }
//           ^ keyword.control
//                   ^ variable

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

// A known divergence: Groovy reads `foo(default: 1)`, a named argument; here
// `default:` is a label, so that `case 1: foo default: bar` keeps its
// `default` case (SPECIFICATION.md §6.8).
foo default: 1
//  ^ label

// `case` is reserved, but a key and a member name keep their captures.
foo case: 1, a: 2
//  ^ variable
//  ^ !keyword.control
bar a: 1, case: 2
//        ^ variable
//        ^ !keyword.control
m = [case: 1]
//   ^ variable
//   ^ !keyword.control
foo(case: 2)
//  ^ variable
//  ^ !keyword.control
x.case
//^ property
x::case
// ^ function

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

def (h, List<Integer> *t) = l
//   ^ variable
//      ^ type
//                    ^ operator
//                     ^ variable

def (name: n, age: int a) = m
//   ^ property
//         ^ variable
//            ^ property
//                 ^ type.builtin
//                     ^ variable

int var = yield
//  ^ variable
//        ^ variable

foo()
in.close()
// <- variable

x = y as String
//    ^ keyword.operator
as = it in xs
// <- variable
//      ^ keyword.operator
return (x) instanceof Foo
//         ^ keyword.operator
//                    ^ type
def kept = (k) in keys
//             ^ keyword.operator
//                ^ variable
def name = (Foo) in
//               ^ variable

var (p, q) = pair()
// <- keyword
var(p, q)
// <- function.call
final var = 1
//    ^ variable

for (v in vs) { }
//     ^ keyword.operator
for (var v in vs) { }
//   ^ keyword

switch (x) { case 1 -> yield 2 }
//                     ^ keyword.control

// A `case` label outside a switch, which Groovy rejects, is a statement that
// misses its `switch`; `case` keeps its capture, and the line after it its own.
case A:
// <- keyword.control
//   ^ variable
foo()
// <- function.call
