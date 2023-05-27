struct Test {
    foo: root::foo::Foo,
}

func test() {
}

func gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let temp = b
        b = a - (a / b) * b
        a = temp
    }
    return a
}
