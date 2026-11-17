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
