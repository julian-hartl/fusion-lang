func extern write(fd: i64, buf: *char, count: i64) -> i64

func print(s: *char) {
    write(1, s, std::string::strlen(s))
}

func println(s: *char) {
    print(s)
    print("\n")
}