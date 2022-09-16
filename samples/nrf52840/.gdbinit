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
monitor rtt stop

echo Flashing...\n
load

monitor reset init

# This scans the entire memory for the RTT control block, which obviously isn't
# ideal. Will cause problems down the line if we ever use flip-link.
monitor rtt setup 0x20000000 262144 "SEGGER RTT"
monitor rtt start

echo Running...\n
continue
