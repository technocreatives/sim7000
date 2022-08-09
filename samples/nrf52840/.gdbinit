# disable "are you sure you want to quit?"
define hook-quit
    set confirm off
end

target extended-remote :3333

# print demangled symbols by default
set print asm-demangle on
set pagination off

monitor arm semihosting enable
monitor reset halt

echo Flashing...\n
load

monitor reset init

echo Running...\n
continue
