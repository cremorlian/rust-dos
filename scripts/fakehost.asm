; fakehost.asm - a DPMI host for the COM1 harness, small enough to read.
;
; Test-only, built and staged by verify_com1_marker.py in place of CWSDPMI.
; Compile-time switches select the host behavior a scenario wants. It implements
; only what the stub exercises.

bits 16
org 0x100

; ---- install. DS is the PSP, which is also this segment at entry. A .COM owns
; all remaining memory, so a second allocation fails; INT 21h/31h keeps this
; segment resident, and the INT 2Fh vector points into it.

start:
    mov dx, int2f_entry            ; handler offset within this segment
    mov ax, 252Fh                  ; INT 21h/25h sets a vector
    int 21h

    mov dx, (resident_end - $$ + 0x100 + 15) / 16  ; paragraphs to keep
    mov ax, 3100h                  ; terminate and stay resident
    int 21h

; ---- resident part: stays in this segment, reached only through CS.

; Mode-switch entry. The client's CS and SS are used; DS/ES stay on this block.
switch_entry:
%ifdef REFUSE_SWITCH
    jmp refuse_switch
%endif

    ; The far call pushed the client's CS at [ss:sp+2]. Read it, then replace
    ; it with the client code selector so retf lands in PM. SP cannot be a
    ; base register in 16-bit addressing, so copy it to BP.
    mov bp, sp
    mov bx, [ss:bp + 2]
    mov word [ss:bp + 2], 0018h    ; client code selector

    ; Descriptors 1-2 cover this segment, 3-4 the client's CS and SS. A 16-bit
    ; shl drops the top bit into the carry, so shift it through with rcl.
    mov ax, cs
    xor dx, dx
    shl ax, 4                      ; DX:AX = this segment's base
    rcl dx, 1
    mov word [cs:gdt_start + 8 + 2], ax
    mov byte [cs:gdt_start + 8 + 4], dl
    mov byte [cs:gdt_start + 8 + 7], 0
    mov word [cs:gdt_start + 16 + 2], ax
    mov byte [cs:gdt_start + 16 + 4], dl
    mov byte [cs:gdt_start + 16 + 7], 0

    mov ax, bx
    xor dx, dx
    shl ax, 4                      ; DX:AX = client CS base
    rcl dx, 1
    mov word [cs:gdt_start + 24 + 2], ax
    mov byte [cs:gdt_start + 24 + 4], dl
    mov byte [cs:gdt_start + 24 + 7], 0

    mov ax, ss
    xor dx, dx
    shl ax, 4                      ; DX:AX = client SS base
    rcl dx, 1
    mov word [cs:gdt_start + 32 + 2], ax
    mov byte [cs:gdt_start + 32 + 4], dl
    mov byte [cs:gdt_start + 32 + 7], 0

    ; GDT, IDT and LDT are all in this segment: each base is the segment base
    ; plus its label, which org 0x100 already makes a segment offset.
    mov ax, cs
    xor dx, dx
    shl ax, 4
    rcl dx, 1
    add ax, gdt_start
    adc dx, 0
    mov word [cs:gdt_ptr + 2], ax
    mov word [cs:gdt_ptr + 4], dx

    mov ax, cs
    xor dx, dx
    shl ax, 4
    rcl dx, 1
    add ax, idt_start
    adc dx, 0
    mov word [cs:idt_ptr + 2], ax
    mov word [cs:idt_ptr + 4], dx

    mov ax, cs
    xor dx, dx
    shl ax, 4
    rcl dx, 1
    add ax, ldt_start
    adc dx, 0
    mov word [cs:gdt_start + 40 + 2], ax
    mov byte [cs:gdt_start + 40 + 4], dl
    mov byte [cs:gdt_start + 40 + 7], 0

    lidt [cs:idt_ptr]
    lgdt [cs:gdt_ptr]

    mov eax, cr0
    or  al, 1                      ; set PE
    mov cr0, eax

    ; Far jump to the host code selector. The offset is the label's segment
    ; offset, because the code descriptor's base is this segment.
    db 0eah                        ; far jump, offset then selector
    dw pm_entry
    dw 0008h

; Entered in protected mode with CS = host code selector.
pm_entry:
    mov ax, 0010h                  ; host data selector
    mov ds, ax
    mov es, ax
    mov ax, 0028h                  ; load the LDT for function 0000h onward
    lldt ax
    mov ax, 0020h                  ; client data selector
    mov ss, ax
    movzx esp, sp                  ; 32-bit client: high word of ESP is zero
    retf

refuse_switch:
    mov ax, 8021h                  ; refused
    stc
    retf

; Unhandled vectors return, so a stray interrupt cannot fault.
default_irq:
    iret

; INT 31h descriptor calls. A 16-bit interrupt gate pushes IP, CS and FLAGS;
; SP cannot be a base register, so BP fixes the saved flags at [ss:bp+6].
int31_handler:
    push bp
    mov bp, sp
    and word [ss:bp + 6], 0FFFEh    ; success: clear the saved carry
    push ds
    push bx
    push cx
    push dx
    push si
    push di
    push ax                         ; the function number
    mov ax, 0010h                   ; host data: writable, same base as CS
    mov ds, ax
    pop ax

%ifdef NO_PRIVATE
    cmp ax, 0300h
    je .stop_after_write
%endif
    cmp ax, 0000h
    je .alloc
    cmp ax, 0003h
    je .increment
    cmp ax, 0009h
    je .rights
    cmp ax, 0007h
    je .base
    cmp ax, 0008h
    je .limit
    jmp .unsupported

.alloc:
%ifdef NO_DESCRIPTORS
    jmp .nodesc
%endif
    mov ax, [ldt_next]
    mov si, ax
    add ax, cx
    cmp ax, 8                       ; only eight descriptors exist
    ja .nodesc
    mov [ldt_next], ax
    shl si, 3                       ; descriptor index to byte offset
    or si, 4                        ; TI=1: the LDT, RPL 0
    mov ax, si
    jmp .done
.nodesc:
    mov ax, 8011h                   ; descriptor unavailable
    jmp .failed

.increment:
    mov ax, 8                       ; one descriptor is eight bytes
    jmp .done

.rights:
%ifdef REJECT_RIGHTS
    jmp .unsupported
%endif
%ifdef REJECT_CODE_RIGHTS
    cmp bx, 4                       ; the stub's first selector is index 0
    jne .unsupported
%endif
    call ldt_entry
    mov [di + 5], cl                ; access rights/type byte
    mov al, [di + 6]
    and al, 00Fh                    ; keep the limit high nibble
    mov ah, ch
    and ah, 0F0h                    ; G, D/B and AVL; low nibble reserved
    or al, ah
    mov [di + 6], al
    jmp .done

.base:
%ifdef REJECT_BASE
    jmp .unsupported
%endif
    call ldt_entry
    mov [di + 2], dx                ; base 15:0
    mov [di + 4], cl                ; base 23:16
    mov [di + 7], ch                ; base 31:24
    jmp .done

.limit:
%ifdef REJECT_LIMIT
    jmp .unsupported
%endif
    call ldt_entry
    movzx eax, cx
    shl eax, 16
    mov ax, dx
    mov edx, eax                    ; EDX = the byte limit
    mov al, [di + 6]
    or al, 080h                     ; FFFF:FFFF is page granular
    shr edx, 12
    mov [di], dx                    ; limit 15:0
    mov ecx, edx
    shr ecx, 16
    and cl, 0Fh                     ; limit 19:16
    and al, 0F0h
    or al, cl
    mov [di + 6], al
    jmp .done

%ifdef NO_PRIVATE
.stop_after_write:
    cmp byte [es:edi + 29], 040h
    jne .unsupported
    cmp word [es:edi + 16], 1
    jne .unsupported
    mov si, msg_nopriv
.next:
    lodsb
    test al, al
    jz .park
    mov ah, al
    mov dx, 03FDh
.wait:
    in al, dx
    test al, 020h
    jz .wait
    mov al, ah
    mov dx, 03F8h
    out dx, al
    jmp .next
.park:
    jmp .park
%endif

.unsupported:
    mov ax, 8001h
.failed:
    or word [ss:bp + 6], 1          ; unsupported: set the saved carry
.done:
    pop di
    pop si
    pop dx
    pop cx
    pop bx
    pop ds
    pop bp
    iret

%ifdef NO_PRIVATE
msg_nopriv: db "NOPRIV", 13, 10, 0
%endif

; DI = the LDT descriptor selected by the selector in BX.
ldt_entry:
    mov di, bx
    and di, 0FFF8h
    add di, ldt_start
    ret

; INT 2Fh / AX=1687h detect.
int2f_entry:
    cmp ax, 1687h
    jne .done

    xor ax, ax                      ; a host is present
%ifdef NO_32BIT
    mov bx, 0                       ; claim no 32-bit client support
%else
    mov bx, 1                       ; bit 0: 32-bit clients supported
%endif
    mov cx, 4                       ; 386 or later
    mov dx, 005Ah                   ; DPMI version 0.90
%ifdef NO_PRIVATE
    xor si, si                      ; the host needs no private data
%elifdef HUNGRY
    mov si, 0FFFFh                  ; more private memory than DOS can supply
%else
    mov si, 1                       ; paragraphs wanted for private memory
%endif

    ; ES:DI is the mode-switch entry, a far pointer into this segment.
    mov di, switch_entry
    push cs
    pop es
.done:
    iret

; ---- GDT: null, host code, host data, client code, client data, LDT. Client
; data is 32-bit; the initial code selector is 16-bit.
;
; A descriptor is eight bytes: limit low, base low, base middle, access, limit
; high with the flags, base high.

gdt_start:
    dq 0                           ; 0: null, a full eight bytes
    dw 0FFFFh                      ; 1: host code: limit 64K
    dw 0                           ;   base, patched
    db 0                           ;   base middle, patched
    db 09Ah                        ;   present, DPL 0, code, readable
    db 00h                         ;   byte granular, 16-bit
    db 0                           ;   base high
    dw 0FFFFh                      ; 2: host data: limit 64K
    dw 0                           ;   base, patched
    db 0                           ;   base middle, patched
    db 092h                        ;   present, DPL 0, data, read/write
    db 00h
    db 0
    dw 0FFFFh                      ; 3: client code: limit 64K
    dw 0                           ;   base, patched
    db 0                           ;   base middle, patched
    db 09Ah
    db 00h                         ;   byte granular, 16-bit
    db 0
    dw 0FFFFh                      ; 4: client data: limit 64K
    dw 0                           ;   base, patched
    db 0                           ;   base middle, patched
    db 092h                        ;   present, DPL 0, data, read/write
    db 040h                        ;   byte granular, 32-bit
    db 0
    dw (ldt_end - ldt_start) - 1   ; 5: LDT: eight descriptors
    dw 0                           ;   base, patched
    db 0                           ;   base middle, patched
    db 082h                        ;   present, DPL 0, LDT
    db 00h                         ;   byte granular
    db 0                           ;   base high, patched
gdt_end:

gdt_ptr:  dw gdt_end - gdt_start - 1
          dd 0                     ; base, patched at switch time

; ---- IDT: 256 16-bit interrupt gates; only 0x31 is handled. Base patched at
; switch time.

idt_start:
%assign v 0
%rep 256
%if v == 0x31
    dw int31_handler
%else
    dw default_irq
%endif
    dw 0008h                       ; host code selector
    db 0
    db 086h                        ; present, DPL 0, 16-bit interrupt gate
    dw 0                           ; offset high
%assign v v+1
%endrep
idt_end:

idt_ptr:  dw idt_end - idt_start - 1
          dd 0                     ; base, patched at switch time

; ---- LDT: eight present data descriptors with base and limit zero, the state
; function 0000h must return.

ldt_start:
%rep 8
    dw 0                           ; limit low
    dw 0                           ; base low
    db 0                           ; base middle
    db 092h                        ; present, DPL 0, data, read/write
    db 0                           ; byte granular, 16-bit
    db 0                           ; base high
%endrep
ldt_end:

ldt_next: dw 0                     ; first free descriptor index

resident_end:
