

func main -> i64 {
    let msg = "Hello, world!"
    std::io::println(msg)
    set_new_msg(&mut msg)
    std::io::println(msg)
    return 0
}

func set_new_msg(msg: *mut *char) {
    *msg = "Goodbye, world!"
}

