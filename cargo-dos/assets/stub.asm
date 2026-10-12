bits 16

; DOS jumps here at offset 0 of the load module, with DS pointing at the PSP
; rather than at this code, hence the CS-relative read below.

    mov si, msg
.lp:
    mov al, [cs:si]
    inc si
    test al, al
    jz done

    mov ah, al                  ; stash: the status read clobbers AL
    mov dx, 03FDh               ; COM1 line status register
.w:
    in al, dx
    test al, 020h               ; bit 5: transmit holding register empty?
    jz .w
    mov al, ah
    mov dx, 03F8h               ; COM1 transmit data register
    out dx, al

    jmp .lp

done:
    jmp $                       ; park; returning to DOS needs the INT 21h bridge

msg: db "stub rm", 13, 10, 0