class Foo { }
//    ^ type.definition

trait Bar { }
//    ^ type.definition

interface I { }
//        ^ type.definition

enum E { }
//   ^ type.definition

record R()
//     ^ type.definition

@interface Marker { }
//         ^ type.definition

@Foo(x = @Bar(1))
// <- attribute
//       ^ attribute
// ^ attribute
//          ^ attribute
class Baz { }

@groovy.transform.CompileStatic
//      ^ attribute
//               ^ attribute
//                 ^ attribute
class Qux { }

def c = String[].class
//      ^ type
//               ^ property

def f = int[]::new
//      ^ type.builtin
//             ^ keyword.operator

@Foo(String[])
//   ^ type
class Arr { }
