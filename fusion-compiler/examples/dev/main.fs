mod baz

func main -> i64 {

    std::io::println("Hello, world!")
    let result = baz::test::gcd(14, 21)
    return result
}