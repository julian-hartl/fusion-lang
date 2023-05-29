.intel_syntax noprefix
.global malloc
.global free

.text

# Signature: func extern malloc(size: i64) -> void*
# size passed in rdi
# return value in rax
malloc:
    mov rax, 12      # Syscall number for sbrk
    syscall          # Call kernel
    ret
# Signature: func extern free(ptr: void*) -> void
# ptr passed in rdi
free:
    ret

