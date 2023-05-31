func extern write(fd: i32, buf: *char, count: isize) -> isize

func print(s: *char) {
    write(1, s, std::string::strlen(s))
}

func println(s: *char) {
    print(s)
    print("\n")
}