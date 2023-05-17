mod foo
mod baz

func main -> i64 {

    let foo = foo::foo()
    let baz = baz::baz()

    return baz.baz

}