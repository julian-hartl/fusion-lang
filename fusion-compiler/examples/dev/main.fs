mod baz

func main -> i64 {

    std::io::println("Hello, world!")
    std::io::println("Hello, world2!")
    std::io::println("Hello, world3!")
    std::io::println("Hello, world4!")
    let result = baz::test::gcd(14, 21)
    return result
}