rm -f withbps.gdb
cargo build --target thumbv7em-none-eabihf
cat src/main.rs | grep -n "add breakpoint here" | sed -r 's;([0-9]+):.*;b main.rs:\1;' > bonusbps
cat openocd.gdb bonusbps > withbps.gdb
gdb-multiarch -x withbps.gdb -q target/thumbv7em-none-eabihf/debug/clocks-and-timers