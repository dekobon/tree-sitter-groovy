'hello'
// <- string

'it\'s a test'
// <- string

'''triple'''
// <- string

'''
multi-line
string'''
// <- string

"hi ${name}"
// <- string
//  ^ punctuation.special
//        ^ punctuation.special

"value $count"
//     ^ variable

"""triple ${expr}"""
// <- string
//        ^ punctuation.special

"lazy ${-> count}"
//     ^ punctuation.special
//       ^ operator

def m = /a+b/ + $/c/d/$ + x / y
//      ^ string
//          ^ string
//              ^ string
//                   ^ string
//                          ^ operator
