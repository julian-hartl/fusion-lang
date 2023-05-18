.intel_syntax noprefix
.global write

.type write, @function
write:
    push rbp
    mov rbp, rsp

    # Signature: func extern write(fd: i64, buf: *char, len: i64) -> i64

    mov rdi, qword ptr [rbp + 16]   # fd
    mov rsi, qword ptr [rbp + 24]   # buf
    mov rdx, qword ptr [rbp + 32]   # len

    mov rax, 1   # System call number for write
    syscall

    mov rbp, rsp
    pop rbp
    ret
