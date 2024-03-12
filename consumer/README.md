# ramlink consumer

This code reads the ramlink buffer of a producer (in this case an ATtiny402) via a jtag2updi dongle using the [AVRICE MKII protocol](https://github.com/Frankkkkk/jtagice-mkii-rs) then translated by the dongle to UPDI (see README of the project).


Please note that you must specify the location of the ring buffer in your devices' sRAM.