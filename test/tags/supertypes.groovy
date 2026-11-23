// tree-sitter-cli 0.27 matches a tag assertion by column only: a later
// tag of the asserted kind that starts at or left of the column
// satisfies it, even on another line. Each line therefore ends in a
// member (`A`, `f`, `m`) whose tag sits right of the supertype, so that
// with the supertype's rule deleted the member's tag comes next and the
// assertion fails.

enum Plain implements Runnable { A }
//   ^ definition.enum
//                    ^ reference.implementation

enum Generic implements Supplier<String> { A }
//                      ^ reference.implementation

enum Qualified implements a.b.Shaded { A }
//                            ^ reference.implementation

class Sub extends pkg.Base<T> { int f }
//    ^ definition.class
//                    ^ reference.class

class Typed extends Parent<T> { int f }
//                  ^ reference.class

class Bare extends Parent { int f }
//                 ^ reference.class

class Dotted extends a.b.Parent { int f }
//                       ^ reference.class

class Impl implements List<String> { int f }
//                    ^ reference.implementation

interface Wide extends a.Far { void m() }
//        ^ definition.interface
//                       ^ reference.implementation

interface Narrow extends Near<D> { void m() }
//                       ^ reference.implementation

interface Flat extends Near { void m() }
//                     ^ reference.implementation

interface Deep extends a.b.Near<D> { void m() }
//                         ^ reference.implementation

trait Mixin implements pkg.Comparable<Mixin> { int f }
//    ^ definition.trait
//                         ^ reference.implementation
