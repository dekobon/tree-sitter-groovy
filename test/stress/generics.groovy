// Stress coverage for SPECIFICATION.md §4 / §5.14 generics.
// Exercises generic_type, type_arguments, type_parameters,
// method_type_parameters, and the wildcard variants in every
// position the grammar currently accepts a `_type`.

import java.util.List
import java.util.Map
import java.util.ArrayList

class Box<T> {
    List<T> contents = []

    def get(T value) { return value }
}

class Pair<A, B> {
    A first = null
    B second = null

    def make(A a, B b) { return [a, b] }
}

interface Holder<T extends Comparable<T>> {
}

trait Boxable<T> {
}

class Util {
    static <T> T identity(T x) {
        return x
    }

    static <T extends Number & Comparable> T pick(T a) {
        return a
    }

    static <K, V> Map<K, V> emptyMap() {
        return [:]
    }
}

def items = new ArrayList<String>()
List<String> names = []
Map<String, Integer> counts = [:]
Map<String, List<Integer>> nested = [:]
List<? extends Number> nums = []
List<? super Integer> sup = []
List<?> any = []
java.util.List<String> qual = []

def cast = (List<String>) other
def ax = obj as List<String>

def closure = { List<String> xs -> xs }
