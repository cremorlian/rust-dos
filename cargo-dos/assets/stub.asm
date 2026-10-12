bits 16
    mov si, msg
.lp:
    mov al, [cs:si]
    inc si
    test al, al
    jz done
    mov ah, al
    mov dx, 03FDh
.w:
    in al, dx
    test al, 020h
    jz .w
    mov al, ah
    mov dx, 03F8h
    out dx, al
    jmp .lp
done:
    jmp $
msg: db "stub rm", 13, 10, 0