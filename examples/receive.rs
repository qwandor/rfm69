use anyhow::Result;
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use linux_embedded_hal::{Delay, Spidev};
use rfm69::registers::{
    DataMode, DioMapping, DioMode, DioPin, DioType, InterPacketRxDelay, Modulation,
    ModulationShaping, ModulationType, PacketConfig, PacketDc, PacketFiltering, PacketFormat,
};
use rfm69::Rfm69;
use utilities::rfm_error;

fn main() -> Result<()> {
    // Configure SPI 8 bits, Mode 0
    let mut spi = Spidev::open("/dev/spidev0.1")?;
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(1_000_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options)?;

    // Create rfm struct with defaults that are set after reset
    let mut rfm = Rfm69::new_without_cs(spi, Delay);

    // Print content of all RFM registers
    for (index, val) in rfm_error!(rfm.read_all_regs())?.iter().enumerate() {
        println!("Register 0x{:02x} = 0x{:02x}", index + 1, val);
    }

    rfm_error!(rfm.frequency(433_850_000.0))?;
    rfm_error!(rfm.bit_rate(3_000.0))?;
    rfm_error!(rfm.preamble(0))?;
    rfm_error!(rfm.sync(&[]))?;

    rfm_error!(rfm.modulation(Modulation {
        data_mode: DataMode::Packet,
        modulation_type: ModulationType::Ook,
        shaping: ModulationShaping::Shaping00,
    }))?;
    rfm_error!(rfm.packet(PacketConfig {
        format: PacketFormat::Variable(10),
        dc: PacketDc::None,
        filtering: PacketFiltering::None,
        crc: false,
        interpacket_rx_delay: InterPacketRxDelay::Delay2Bits,
        auto_rx_restart: true,
    }))?;
    rfm_error!(rfm.dio_mapping(DioMapping {
        pin: DioPin::Dio2,
        dio_type: DioType::Dio01, // Data
        dio_mode: DioMode::Rx,
    }))?;
    rfm_error!(rfm.dio_mapping(DioMapping {
        pin: DioPin::Dio3,
        dio_type: DioType::Dio01, // RSSI
        dio_mode: DioMode::Rx,
    }))?;

    // Prepare buffer to store the received data
    let mut buffer = [0; 64];
    rfm_error!(rfm.recv(&mut buffer))?;
    // Print received data
    for (index, val) in buffer.iter().enumerate() {
        println!("Value at {} = {}", index, val);
    }

    Ok(())
}
