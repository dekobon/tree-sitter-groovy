// As in supertypes.groovy, each asserted tag has a tag of another rule
// right of it on its line, because an assertion is matched by column
// only: `new Z()` after a call, `z()` after an instantiation.

foo(new Z())
// <- reference.call

obj.method(new Z())
//  ^ reference.call

obj?.method(new Z())
//   ^ reference.call

obj??.method(new Z())
//    ^ reference.call

list*.method(new Z())
//    ^ reference.call

obj.&method(new Z())
//   ^ reference.call

sh 'make', new Z()
// <- reference.call

def out = sh 'make', new Z()
//        ^ reference.call

obj.method 'x', new Z()
//  ^ reference.call

a.b.method 1, new Z()
//  ^ reference.call

obj?.method 'x', new Z()
//   ^ reference.call

obj??.method 'x', new Z()
//    ^ reference.call

list*.method 'x', new Z()
//    ^ reference.call

obj.&method 'x', new Z()
//   ^ reference.call

println new Z()
// <- reference.call
//          ^ reference.class

obj.method new Z()
//  ^ reference.call

def r = sh new Z()
//      ^ reference.call

x = (sh 'make', new Z())
//   ^ reference.call

if (fileExists 'x', new Z()) { }
//  ^ reference.call

def m = "${sh 'make', new Z()}"
//         ^ reference.call

new Foo(z())
//  ^ reference.class

new Box<String>(z())
//  ^ reference.class
