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
