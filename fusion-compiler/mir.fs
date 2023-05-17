func extern write(fd: i64, buf: *char, len: i64) -> i64

func main -> i64 {
    let buf = "Hello, world!\n"
    write(1, buf, 14)
    return 0
}