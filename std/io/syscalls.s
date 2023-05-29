.intel_syntax noprefix
.global write

# Signature: func extern write(fd: i64, buf: *char, len: i64) -> i64
write:
    push rbp
    mov rbp, rsp


    mov rax, 1   # System call number for write
    syscall

    mov rbp, rsp
    pop rbp
    ret
