# Native 32-bit timing oracle (GNU as): oracle.asm's call of the formula
# code, repeated.  Header as in oracle.asm, plus
#   [img+44] repeats  [img+48] region 1 address  [img+52] its length (dwords)
#   [img+56] region 2 address  [img+60] its length (dwords)
# The two regions (the iteration record's values the formula changes) are
# restored before every call; the nanoseconds of the whole loop are written
# to [img+64] (low) and [img+68] (high) of the image sent back.
# Build: as --32 -o oracle_bench.o oracle_bench.s && ld -m elf_i386 -o oracle_bench oracle_bench.o
        .intel_syntax noprefix
        .set BASE, 0x10000000
        .set SIZE, 0x10000
        .globl _start
        .bss
saved_esp: .long 0
count: .long 0
save1: .space 1024
save2: .space 1024
ts0: .long 0, 0
ts1: .long 0, 0
        .text
_start:
        mov eax, 192
        mov ebx, BASE
        mov ecx, SIZE
        mov edx, 7
        mov esi, 0x32
        mov edi, -1
        xor ebp, ebp
        int 0x80
        cmp eax, BASE
        jne fail
        xor esi, esi
rd:
        mov eax, 3
        xor ebx, ebx
        lea ecx, [BASE + esi]
        mov edx, SIZE
        sub edx, esi
        jz rd_done
        int 0x80
        test eax, eax
        jle rd_done
        add esi, eax
        jmp rd
rd_done:
        cmp esi, SIZE
        jne fail
        # the saved regions
        mov esi, [BASE + 48]
        lea edi, save1
        mov ecx, [BASE + 52]
        rep movsd
        mov esi, [BASE + 56]
        lea edi, save2
        mov ecx, [BASE + 60]
        rep movsd
        mov eax, [BASE + 44]
        mov count, eax
        finit
        mov eax, 265            # clock_gettime(CLOCK_MONOTONIC)
        mov ebx, 1
        lea ecx, ts0
        int 0x80
loop_top:
        cmp dword ptr count, 0
        je loop_end
        dec dword ptr count
        lea esi, save1
        mov edi, [BASE + 48]
        mov ecx, [BASE + 52]
        rep movsd
        lea esi, save2
        mov edi, [BASE + 56]
        mov ecx, [BASE + 60]
        rep movsd
        mov saved_esp, esp
        mov esp, [BASE + 24]
        cmp dword ptr [BASE + 28], 1
        jne call_std
        mov esi, [BASE + 32]
        mov edi, [BASE + 36]
        mov ebx, [BASE + 40]
        mov ecx, [BASE + 12]
        lea edx, [esi + 128]
        xor eax, eax
        call dword ptr [BASE + 0]
        jmp call_back
call_std:
        push dword ptr [BASE + 16]
        push dword ptr [BASE + 20]
        mov eax, [BASE + 4]
        mov edx, [BASE + 8]
        mov ecx, [BASE + 12]
        call dword ptr [BASE + 0]
call_back:
        mov esp, saved_esp
        jmp loop_top
loop_end:
        mov eax, 265
        mov ebx, 1
        lea ecx, ts1
        int 0x80
        # ns = (s1 - s0) * 1e9 + (ns1 - ns0), as a 64-bit value
        mov eax, ts1
        sub eax, ts0
        mov ecx, 1000000000
        mul ecx
        mov ebx, ts1 + 4
        sub ebx, ts0 + 4
        mov ecx, ebx
        sar ecx, 31
        add eax, ebx
        adc edx, ecx
        mov [BASE + 64], eax
        mov [BASE + 68], edx
        xor esi, esi
wr:
        mov eax, 4
        mov ebx, 1
        lea ecx, [BASE + esi]
        mov edx, SIZE
        sub edx, esi
        jz done
        int 0x80
        test eax, eax
        jle fail
        add esi, eax
        jmp wr
done:
        mov eax, 1
        xor ebx, ebx
        int 0x80
fail:
        mov eax, 1
        mov ebx, 3
        int 0x80
