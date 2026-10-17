bits 16

; DOS jumps here at offset 0 of the load module, with DS pointing at the PSP
; rather than at this code, hence the CS-relative read below.

    mov [cs:loadseg], cs        ; real mode allows a write through CS
    mov si, msg_rm
    call puts

    mov ax, 1687h               ; DPMI detect
    int 2fh
    test ax, ax
    jnz .nodpmi
    test bl, 1                  ; bit 0: host offers 32-bit clients
    jnz .have32
    mov si, msg_no32            ; a capable host may still report no, so warn
    call puts                   ; and try regardless: the switch is the authority
.have32:
    mov [cs:entry], di          ; the far mode-switch entry, before DOS can
    mov [cs:entry + 2], es      ; clobber ES:DI

    mov bx, si                  ; private paragraphs the host asked for
    mov ah, 48h                 ; DOS allocate
    int 21h
    jc .nomem
    mov es, ax

    mov ax, 1                   ; 32-bit client
    call far [cs:entry]
    jc .noswitch

    mov si, msg_pm
    call puts

; ---- 16-bit protected mode: build the image's descriptors

;   The DPMI switch leaves us in a 16-bit segment, so the 32-bit instructions
;   here and in the far jump below each carry their own operand-size prefix;
;   use32 would suppress them. SS and DS come back pointing at the host's own
;   data segment, 32-bit-default.

;   The host's data segment is the only writable memory before we own a
;   descriptor, and 0000h and 0003h return through AX, which every later call
;   overwrites with its function number. So the selectors are parked near the top
;   of that segment, which is safe only because a host keeps its TSS stack low.

    xor ax, ax
    mov cx, 2                   ; a code and a data descriptor, contiguous
    int 31h
    jc .nodesc                  ; host could not allocate LDT descriptors
    mov [ss:0FFC0h], ax         ; see above: this is where the selectors live

    mov ax, 0003h               ; selector increment, which cannot fail
    int 31h
    mov [ss:0FFC2h], ax

    mov bx, [ss:0FFC0h]
    mov cl, 0F2h                ; present, DPL 3, data, read/write
    mov ch, 0C0h                ; page granular, default 32-bit
    mov ax, 0009h
    int 31h

    mov bx, [ss:0FFC0h]         ; data plus the increment is the code one
    add bx, [ss:0FFC2h]
    mov cl, 0FAh                ; present, DPL 3, code, readable
    mov ch, 0C0h
    mov ax, 0009h
    int 31h

;   Base both at the image start, loadseg*16 + IMAGE_OFF. The CPU adds a
;   segment base to every offset, so the image's image-relative addresses
;   resolve through DS as linked and the fixup table is never applied.
    mov bp, [cs:loadseg]
    movzx eax, bp               ; base = loadseg*16 + IMAGE_OFF. A 16-bit shift
    shl eax, 4                  ; would truncate: loadseg*16 overflows 16 bits
    add eax, IMAGE_OFF          ; once loadseg passes 1000h
    mov dx, ax
    shr eax, 16
    mov cx, ax                  ; the base arrives as CX:DX, CX being the high word
    mov bx, [ss:0FFC0h]
    mov ax, 0007h
    int 31h
    mov bx, [ss:0FFC0h]
    add bx, [ss:0FFC2h]
    mov ax, 0007h
    int 31h

    mov cx, 0FFFFh              ; 4 GB
    mov dx, 0FFFFh
    mov bx, [ss:0FFC0h]
    mov ax, 0008h
    int 31h
    mov bx, [ss:0FFC0h]
    add bx, [ss:0FFC2h]
    mov ax, 0008h
    int 31h

    mov si, [ss:0FFC0h]         ; the data selector, and the code selector is
    mov bp, si                  ; this one plus the increment
    add bp, [ss:0FFC2h]

    mov word [ss:0FFC4h], 0     ; a far pointer: eip 0, then the code selector.
    mov word [ss:0FFC6h], 0     ; Two word stores, not one dword: nasm gives
    mov [ss:0FFC8h], bp         ; `mov dword [...], 0` a 2-byte immediate here
    mov ds, si
    mov es, si

;   A far jump, not a far return. The target selector has RPL 3 and so does our
;   CPL, so retf would not pop SS and ESP either; a jump just needs no stack.
    db 66h
    jmp far [ss:0FFC4h]

.noswitch:
    mov si, msg_noswitch
    jmp .fail

.nodpmi:
    mov si, msg_nodpmi
    jmp .fail

.nomem:
    mov si, msg_nomem
    jmp .fail

.nodesc:
    mov si, msg_nodesc
    jmp .fail

.fail:
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
loadseg: dw 0
msg_rm: db "stub rm", 13, 10, 0
msg_pm: db "stub pm", 13, 10, 0
msg_nodpmi: db "rust-dos: no DPMI host found", 13, 10, 0
msg_no32: db "rust-dos: warning: host reports no 32-bit support", 13, 10, 0
msg_nomem: db "rust-dos: insufficient memory for the DPMI host", 13, 10, 0
msg_nodesc: db "rust-dos: no LDT descriptors available", 13, 10, 0
msg_noswitch: db "rust-dos: DPMI host refused mode switch", 13, 10, 0

IMAGE_OFF equ $ - $$
