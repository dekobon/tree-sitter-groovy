enum Plain implements Runnable { A }
//   ^ definition.enum
//                    ^ reference.implementation

enum Generic implements Supplier<String> { A }
//                      ^ reference.implementation

enum Qualified implements a.b.Shaded { A }
//                            ^ reference.implementation

class Sub extends pkg.Base<T> { }
//    ^ definition.class
//                    ^ reference.class

class Typed extends Parent<T> { }
//                  ^ reference.class

class Impl implements List<String> { }
//                    ^ reference.implementation

interface Wide extends a.Far { }
//        ^ definition.interface
//                       ^ reference.implementation

interface Narrow extends Near<D> { }
//                       ^ reference.implementation

trait Mixin implements pkg.Comparable<Mixin> { }
//    ^ definition.trait
//                         ^ reference.implementation
