// As in supertypes.groovy, each asserted tag has a tag of another rule
// right of it on its line, because an assertion is matched by column
// only.

@interface Marker { String value() }
//         ^ definition.interface

class Counter { int count; def run() { } }
//                  ^ definition.field

class Runner { def run() { }; int count }
//                 ^ definition.method

enum Mode { ON; int code }
//          ^ definition.constant

interface Greeter { default String greet() { 'hi' }; int count }
//                                 ^ definition.method

@Memoized fib(n) { n }; class Fib { }
//        ^ definition.method

private <T> first(List<T> xs) { xs }; class First { }
//          ^ definition.method
