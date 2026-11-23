record Point(int x) {
    Point(int x) { }
    // <- definition.constructor
}

record Warning(String message) {
//     ^ definition.class
    public Warning {
    //     ^ definition.constructor
        assert message
    }
}
