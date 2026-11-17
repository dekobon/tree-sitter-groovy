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
class Baz { }
