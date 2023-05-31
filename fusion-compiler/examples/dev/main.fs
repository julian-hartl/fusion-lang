
func main -> i64 {
    let mut i = 10
    while i > 0 {
        std::io::println("Hello, world!");
        i = i - 1
    }
    i = 10
    while i > 0 {
            std::io::println("Hello, world!");
            i = i - 1
    }
    let f = Foo { x: 10, y: 20 }
    return f.x
}

struct Foo {
    x: i64,
    y: i64,
}


