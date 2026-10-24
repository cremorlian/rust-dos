bits 16

; DOS jumps here at offset 0 of the load module, with DS pointing at the PSP
; rather than at this code, hence the CS-relative read below.

    mov [cs:loadseg], cs        ; real mode allows a write through CS

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
    test bx, bx                 ; zero: no data area, and ES is ignored
    jz .have_private
    mov ah, 48h                 ; DOS allocate
    int 21h
    jc .nomem
    mov es, ax
.have_private:

    mov [cs:saved_ss], ss
    mov [cs:saved_sp], sp

    mov ax, 1                   ; 32-bit client
    call far [cs:entry]
    jc .noswitch

; ---- 16-bit protected mode: build the image's descriptors

;   The DPMI switch leaves us in a 16-bit segment, so the 32-bit instructions
;   here and in the far jump below each carry their own operand-size prefix;
;   use32 would suppress them.

;   Before we own a descriptor there is no writable data segment of our own, so
;   the two values 0000h and 0003h return through AX are moved into registers
;   that INT 31h preserves. The far pointer below is built on the stack, so no
;   fixed host-segment address is involved.

    xor ax, ax
    mov cx, 2                   ; a code and a data descriptor, contiguous
    int 31h
    jc .nodesc                  ; host could not allocate LDT descriptors
    mov si, ax                  ; the data selector; 0000h returns it in AX

    mov ax, 0003h               ; selector increment, which cannot fail
    int 31h
    mov di, ax                  ; the code selector is SI + DI

    ; DPMI 0.9 5.2: a client must not assume the ring it runs at. Read the
    ; DPL of the code segment the switch returned and use that ring for the
    ; descriptors, so the far jump below stays at the current privilege level.
    mov dx, cs
    lar ax, dx                  ; AX = access rights of the initial CS
    mov bp, ax
    shr bp, 13                  ; the access byte is bits 8-15; DPL is its 5-6
    and bp, 3
    shl bp, 5                   ; BP = the DPL field of an access byte

    mov bx, si
    mov cl, 092h                ; present, data, read/write
    mov ax, bp
    or cl, al                   ; plus the client's own DPL
    mov ch, 0C0h                ; page granular, default 32-bit
    mov ax, 0009h
    int 31h
    jc .rights_fail

    mov bx, si                  ; data plus the increment is the code one
    add bx, di
    mov cl, 09Ah                ; present, code, readable
    mov ax, bp
    or cl, al
    mov ch, 0C0h
    mov ax, 0009h
    int 31h
    jc .rights_fail

;   Both descriptors are flat. The image's linked offsets become linear
;   addresses through the relocation pass before any image code runs.
    xor cx, cx
    xor dx, dx
    mov bx, si
    mov ax, 0007h
    int 31h
    jc .desc_fail

    xor cx, cx
    xor dx, dx
    mov bx, si
    add bx, di
    mov ax, 0007h
    int 31h
    jc .desc_fail

    mov cx, 0FFFFh              ; 4 GB
    mov dx, 0FFFFh
    mov bx, si
    mov ax, 0008h
    int 31h
    jc .desc_fail
    mov bx, si
    add bx, di
    mov ax, 0008h
    int 31h
    jc .desc_fail

    mov ds, si
    mov es, si

;   Apply the image's R_386_32 relocations through the flat data selector.
;   TABLE_OFF is module-relative; the table itself follows the image.
    mov bp, [cs:loadseg]
    movzx ebx, bp
    shl ebx, 4
    add ebx, IMAGE_OFF          ; EBX = image load address
    movzx edx, bp
    shl edx, 4
    mov eax, [cs:table_off]
    add edx, eax                ; EDX = fixup table address
.reloc:
    mov eax, [edx]
    cmp eax, 0FFFFFFFFh
    je .reloc_done
    add eax, ebx
    add dword [eax], ebx
    add edx, 4
    jmp .reloc
.reloc_done:

    mov bp, si                  ; the code selector is the data one plus the
    add bp, di                  ; increment

;   Switch to the saved real-mode stack, now addressed linearly.
    movzx eax, word [cs:saved_ss]
    shl eax, 4
    movzx edx, word [cs:saved_sp]
    add eax, edx
    mov ss, si
    mov esp, eax

;   A far pointer on the stack: selector, then the 32-bit linear entry address.
;   EBP finds it, so nothing is stored at a fixed offset in the host's segment.
    push bp
    push ebx                    ; the image's load address, from the relocation
    mov ebp, esp

;   A far jump, not a far return. The target selector's RPL equals our CPL,
;   so retf would not switch stacks either; the jump leaves the pointer on the
;   stack instead of popping it.
    db 66h
    jmp far [ss:ebp]

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

.rights_fail:
    mov si, msg_rights
    jmp .fail

.desc_fail:
    mov si, msg_desc
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
saved_ss: dw 0
saved_sp: dw 0
msg_nodpmi: db "rust-dos: no DPMI host found", 13, 10, 0
msg_no32: db "rust-dos: warning: host reports no 32-bit support", 13, 10, 0
msg_nomem: db "rust-dos: insufficient memory for the DPMI host", 13, 10, 0
msg_nodesc: db "rust-dos: no LDT descriptors available", 13, 10, 0
msg_rights: db "rust-dos: DPMI host rejected descriptor rights", 13, 10, 0
msg_desc: db "rust-dos: DPMI host rejected descriptor setup", 13, 10, 0
msg_noswitch: db "rust-dos: DPMI host refused mode switch", 13, 10, 0

table_off: dd 0

IMAGE_OFF equ $ - $$
