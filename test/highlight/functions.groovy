def greet() { }
//  ^ function

def add(a, b) { a + b }
//  ^ function

foo()
// <- function.call

doIt(a, b)
// <- function.call

obj.method()
//  ^ function.call

def out = sh script: 'make'
//        ^ function.call

x = foo 1
//  ^ function.call

obj.method 'x'
//  ^ function.call

def v = a.b.method 1
//          ^ function.call
//        ^ property

println new Date()
// <- function.call
//      ^ keyword.operator
//          ^ type

log.info new Foo()
//  ^ function.call

def r = foo new Foo()
//      ^ function.call

def p = (log.info 'x')
//           ^ function.call

if (fileExists 'pom.xml') { }
//  ^ function.call

def m = "v: ${sh script: 'x'}"
//             ^ function.call

println(foo 1)
// <- function.call
//      ^ function.call

x.add(foo 'a')
//^ function.call
//    ^ function.call

println x
// <- function.call

println "a" as String
// <- function.call
//          ^ keyword.operator
//             ^ type

assertEquals expected, actual
// <- function.call

def r = foo bar
//      ^ function.call

obj.method arg
//  ^ function.call

then: thrown IllegalStateException
//    ^ function.call

obj?.method(x)
//   ^ function.call

obj??.method(x)
//    ^ function.call

list*.method(x)
//    ^ function.call

obj?.method 'x'
//   ^ function.call

obj??.method 'x'
//    ^ function.call

list*.method 'x'
//    ^ function.call

obj?.prop
//   ^ property

obj??.prop
//    ^ property

list*.prop
//    ^ property

obj.&method(x)
//   ^ function

obj.@field(x)
//   ^ property

obj.@field 'x'
//   ^ property

obj?.@field
//    ^ property

obj??.@field
//     ^ property

list*.@field
//     ^ property

list*.@field
//  ^ operator

String::length
//      ^ function

obj.&method
//  ^ operator

def r = new Runnable() {
    int count = 0
    //  ^ property
    void run() { }
    //   ^ function
}

record Warning(String message) {
    public Warning {
    //     ^ constructor
        assert message
    }
}

def inc = x -> x + 1
//        ^ variable.parameter
//          ^ operator

list.sort((a, b) -> a <=> b)
//         ^ variable.parameter
//            ^ variable.parameter
//               ^ operator

foo(item -> item.name)
//  ^ variable.parameter
//                ^ property

def c = { a = b -> a }
//        ^ variable.parameter
//            ^ variable

foo { }.bar()
// <- function.call
//      ^ function.call

foo { }.bar 1
// <- function.call
//      ^ function.call
