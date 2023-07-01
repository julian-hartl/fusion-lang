func strlen(s: *char) -> isize {
    let mut length = 0isize;
    while s[length] != '\0' {
        length = length + 1isize;
    }
    return length;
}

func strcpy(dest: *mut char, src: *char) {
    let mut i = 0isize;
    while src[i] != '\0' {
        dest[i] = src[i];
        i = i + 1isize;
    }
    dest[i] = '\0';
}

func strcat(dest: *mut char, src: *char) {
    let dest_len = strlen(dest);
    let mut i = 0isize;
    while src[i] != '\0' {
        dest[dest_len + i] = src[i];
        i = i + 1isize;
    }
    dest[dest_len + i] = '\0';
}

func strcmp(str1: *char, str2: *char) -> bool {
    let mut i = 0isize;
    while str1[i] != '\0' && str2[i] != '\0' && str1[i] == str2[i] {
        i = i + 1isize;
    }
    return str1[i] - str2[i] == '\0';
}


