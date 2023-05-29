

func main -> i64 {

}

func strlen(s: *char) -> i64 {
    let mut length = 0
    while s[length] != '\0' {
        length = length + 1
    }
    return length
}


