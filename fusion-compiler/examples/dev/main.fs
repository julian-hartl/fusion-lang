struct Program {
    text: *char,
}

func main -> i64 {
    let msg = "Hello, world!"
    std::io::println(msg)
    set_new_msg(&mut msg)
    std::io::println(msg)
    let mut program = Program { text: msg }
    program.text = "Goodbye, world!"
    std::io::println(program.text)
    return 0
}

func set_new_msg(msg: *mut *char) {
    *msg = "Goodbye, world!"
}

