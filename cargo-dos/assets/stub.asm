bits 16

; DOS jumps here at offset 0 of the load module, with DS pointing at the PSP
; rather than at this code, hence the CS-relative read below.

    mov si, msg_rm
    call puts

    mov ax, 1687h               ; DPMI detect
    int 2fh
    test ax, ax
    jnz park
    test bl, 1                  ; bit 0: host can enter 32-bit
    jz park

    mov [cs:entry], di          ; the far mode-switch entry, before DOS can
    mov [cs:entry + 2], es      ; clobber ES:DI

    mov bx, si                  ; private paragraphs the host asked for
    mov ah, 48h                 ; DOS allocate
    int 21h
    jc park
    mov es, ax

    mov ax, 1                   ; 32-bit client
    call far [cs:entry]

    mov si, msg_pm
    call puts

park:
    jmp $                       ; park; returning to DOS needs the INT 21h bridge

; ---- polled COM1 output, CS-relative so it works before and after the switch

putc:                            ; character in AL
    mov ah, al                  ; stash: the status read clobbers AL
    mov dx, 03FDh               ; COM1 line status register
.w:
    in al, dx
    test al, 020h               ; bit 5: transmit holding register empty?
    jz .w
    mov al, ah
    mov dx, 03F8h               ; COM1 transmit data register
    out dx, al
    ret

puts:                            ; string at CS:SI; SI advances, AX and DX clobbered
.lp:
    mov al, [cs:si]
    inc si
    test al, al
    jz .done
    call putc
    jmp .lp
.done:
    ret

entry:  dw 0, 0
msg_rm: db "stub rm", 13, 10, 0
msg_pm: db "stub pm", 13, 10, 0
