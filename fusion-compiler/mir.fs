func extern write(fd: i64, buf: *char, len: i64) -> i64
func fib(n: i64) -> i64 {
    if n <= 1 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}
func main -> i64 {
    return fib(8)
}