struct Program {
    text: *char,
}

func main -> i64 {
    let mut i = 10
    while i > 0 {
        let msg = "Hello, world!"
        let p = Program { text: msg }
        if std::string::strcmp(msg, p.text) {
            std::io::println("Strings are equal")
        } else {
            std::io::println("Strings are not equal")
        }
        std::io::println(p.text)
        set_new_msg(&mut p.text)
        std::io::println(p.text)
        i = i - 1
    }
    return i + 1
}

func set_new_msg(msg: *mut *char) {
    *msg = "Goodbye, world!"
}

