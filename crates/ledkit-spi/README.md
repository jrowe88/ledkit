# ledkit-spi

A ledkit backend that drives NeoPixels (WS2812B and friends) over any
`embedded-hal` 1.0 SPI bus, blocking or async. Work in progress.

You set up the SPI peripheral, its clock and the data pin; ledkit never
configures the bus. Then you hand over the bus and its real clock rate.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option. See NOTICE for attributions.
