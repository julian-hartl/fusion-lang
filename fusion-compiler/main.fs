struct Node {
    val: i64
    next: *Node
}

struct List {
    head: *Node
    tail: *Node
}

func List_new() -> *mut List {
    let list = malloc(16) as *mut List
    list.head = 0 as *Node
    list.tail = 0 as *Node
    return list
}

func List_push(list: *mut List, val: i64) {
    let node = malloc(16) as *mut Node
    node.val = val
    node.next = 0 as *Node
    if list.head == 0 as *Node {
        list.head = node
        list.tail = node
    } else {
        list.tail.next = node
        list.tail = node
    }
}



func List_print(list: *List) {
    let mut node = list.head
    while node != 0 as *Node {
        printf(itoa(node as i64))
        printf("\n")
        node = node.next
    }
    printf("\n")
}


func extern printf(s: *char)
func extern malloc(size: i64) -> *mut void
func extern free(ptr: *void) 
func itoa(i: i64) -> *char {
    let buf = malloc(20) as *mut char
    let mut len = 0
    let mut temp = i

    if i == 0 {
        *buf = '0';
        *(buf + 1) = 0
        return buf
    }

    while temp != 0 {
        temp = temp / 10
        len = len + 1
    }

    let index = len - 1
    let mut num = i

    let mut j = 0
    while j < len {
        let digit = num % 10
        num = num / 10;
        *(buf + index - j) = digit as char + '0'
        j = j + 1
    }

    *(buf + len) = 0
    return buf
}

func set_by_ref(i: *mut i64, val: i64) {
    *i = val
}
func set_by_value(mut i: i64, val: i64) {
    i = val
}
struct Foo {
    a: i64
    b: i64
}

func Foo_create(a: i64, b: i64) -> *mut Foo {
    let foo = &mut Foo {
            a: 100
            b: 200
        }
    return foo
}

func main -> i64 {

    let foo = Foo {
                a: 100
                b: 200
            }

    printf(itoa(foo->a))
    printf("\n")
    printf(itoa(foo->b))
    printf("\n")

    return 0
}
func fib(n: i64) -> i64 {
    if n <= 1 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}
func gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let temp = b
        b = a - (a / b) * b
        a = temp
    }
    return a
}
