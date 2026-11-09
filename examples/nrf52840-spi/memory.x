/* Seeed XIAO nRF52840 with its factory UF2 bootloader (Adafruit nRF52
 * bootloader with the S140 7.3.0 SoftDevice). The SoftDevice takes the flash
 * below 0x27000; from 0xED000 come a user-data area (Arduino's file system)
 * and, from 0xF4000, the bootloader itself. The program goes in between.
 *
 * RAM: this program doesn't turn on the SoftDevice (no Bluetooth), but the
 * first 8 bytes of RAM still belong to the MBR and SoftDevice: they hold the
 * address that interrupts are forwarded to. Writing over them crashes the
 * board at its first interrupt. A program that enables the SoftDevice must
 * start RAM after the SoftDevice's share (nrf-softdevice prints it). */
MEMORY
{
  FLASH : ORIGIN = 0x00027000, LENGTH = 0xED000 - 0x27000
  RAM : ORIGIN = 0x20000008, LENGTH = 256K - 8
}
