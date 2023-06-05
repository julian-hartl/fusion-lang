
func main -> i32 {
    let mut i = 10
    while i > 0 {
        std::io::println("Hello, world!");
        i = i - 1
    }
    i = 10
    while i > 0 {
            std::io::println("Hello, world2!");
            i = i - 1
    }
    let f = Foo { x: 10, y: 20 }
    if is_zero(f.x) {
        std::io::println("x is zero")
    } else {
        std::io::println("x is not zero")
    }
    let a = 10
    let b = 10
    let c = 10;
    do_stuff(a, b) + c
    return c
}

struct Foo {
    x: i32,
    y: i32
}

func do_stuff(a: i32, b: i32) -> i32 {
    let a = 10
    let b = 10
    let c = 20
    let d = 100
    let e = 200
    return e
}

func add(a: i32, b: i32) -> i32 {
    return a + b
}

func is_zero(a: i32) -> bool {
    return a == 0
}

