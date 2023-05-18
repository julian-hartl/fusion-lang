mod baz

func main -> i64 {

    std::io::write(1, "Hello, world!\n", 14)
    let result = baz::test::gcd(14, 21)
    return result
}