struct Program {
    text: *char,
}

func main -> i64 {
    let mut i = 10
    while i > 0 {
        let msg = "Hello, world!"
        std::io::println(msg)
        i = i - 1
    }
    return i + 1
}

func set_new_msg(msg: *mut *char) {
    *msg = "Goodbye, world!"
}

