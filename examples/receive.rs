use anyhow::Result;
use embedded_hal::digital::OutputPin;
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use linux_embedded_hal::sysfs_gpio::Direction;
use linux_embedded_hal::{SpidevDevice, SysfsPin};
use rfm69::registers::{
    DataMode, DioMapping, DioMode, DioPin, DioType, InterPacketRxDelay, Modulation,
    ModulationShaping, ModulationType, PacketConfig, PacketDc, PacketFiltering, PacketFormat,
};
use rfm69::Rfm69;
use std::thread::sleep;
use std::time::Duration;
use utilities::rfm_error;

fn main() -> Result<()> {
    let mut reset = SysfsPin::new(25);
    reset.export()?;
    reset.set_direction(Direction::Low)?;
    reset.set_high()?;
    sleep(Duration::from_millis(1));
    reset.set_low()?;
    sleep(Duration::from_millis(5));

    // Configure SPI 8 bits, Mode 0
    let mut spi = SpidevDevice::open("/dev/spidev0.1")?;
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(1_000_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options)?;

    // Create rfm struct with defaults that are set after reset
    let mut rfm = Rfm69::new(spi);

    // Print content of all RFM registers
    for (index, val) in rfm_error!(rfm.read_all_regs())?.iter().enumerate() {
        //println!("Register 0x{:02x} = 0x{:02x}", index + 1, val);
    }

    rfm_error!(rfm.frequency(433_850_000))?;
    rfm_error!(rfm.bit_rate(3_000))?;
    // TODO: Configure automatic frequency correction
    rfm_error!(rfm.rssi_threshold(220))?;
    //rfm_error!(rfm.lna(0x88))?;
    //rfm_error!(rfm.rx_bw(0x55))?;
    //rfm_error!(rfm.rx_afc_bw(0x8b))?;
    rfm_error!(rfm.preamble(0))?;
    rfm_error!(rfm.sync(&[0x8e]))?;

    rfm_error!(rfm.modulation(Modulation {
        data_mode: DataMode::Packet,
        modulation_type: ModulationType::Ook,
        shaping: ModulationShaping::Shaping00,
    }))?;
    rfm_error!(rfm.packet(PacketConfig {
        format: PacketFormat::Fixed(0),
        dc: PacketDc::None,
        filtering: PacketFiltering::None,
        crc: false,
        interpacket_rx_delay: InterPacketRxDelay::Delay2Bits,
        auto_rx_restart: false,
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
    for val in buffer.iter() {
        print!("{:02x} ", val);
        if *val == 0 {
            println!();
        }
    }
    println!();
    for val in buffer.iter() {
        print!("{:08b} ", val);
        if *val == 0 {
            println!();
        }
    }
    println!();

    Ok(())
}
