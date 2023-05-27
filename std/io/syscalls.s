.intel_syntax noprefix
.global write

write:
    push rbp
    mov rbp, rsp

    # Signature: func extern write(fd: i64, buf: *char, len: i64) -> i64

    mov rax, 1   # System call number for write
    syscall

    mov rbp, rsp
    pop rbp
    ret
