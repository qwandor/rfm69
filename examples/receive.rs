use anyhow::Result;
use embedded_hal::digital::v2::OutputPin;
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use linux_embedded_hal::sysfs_gpio::Direction;
use linux_embedded_hal::{Delay, Spidev, SysfsPin};
use rfm69::registers::{
    DataMode, DccCutoff, DioMapping, DioMode, DioPin, DioType, FifoMode, InterPacketRxDelay,
    LnaConfig, LnaGain, LnaImpedance, Mode, Modulation, ModulationShaping, ModulationType,
    PacketConfig, PacketDc, PacketFiltering, PacketFormat, Registers, RxBw, RxBwOok,
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
    let mut spi = Spidev::open("/dev/spidev0.1")?;
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(1_000_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options)?;

    // Create rfm struct with defaults that are set after reset
    let mut rfm = Rfm69::new_without_cs(spi, Delay);

    let sampling_rate = 15_000.0; // in Hz
    rfm_error!(rfm.frequency(433_850_000.0))?;
    rfm_error!(rfm.bit_rate(sampling_rate))?;
    // TODO: Configure automatic frequency correction
    // Actual RSSI threshold is -N/2.
    rfm_error!(rfm.rssi_threshold(175))?;
    rfm_error!(rfm.lna(LnaConfig {
        zin: LnaImpedance::Ohm200,
        gain_select: LnaGain::AgcLoop,
    }))?;
    rfm_error!(rfm.rx_bw(RxBw {
        dcc_cutoff: DccCutoff::Percent4,
        rx_bw: RxBwOok::Khz200dot0
    }))?;
    //rfm_error!(rfm.rx_afc_bw(0x8b))?;
    rfm_error!(rfm.preamble(0))?;
    rfm_error!(rfm.sync(&[0x0f]))?;
    rfm_error!(rfm.fifo_mode(FifoMode::Level(2)))?;

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

    rfm_error!(rfm.mode(Mode::Receiver))?;
    rfm_error!(rfm.wait_mode_ready())?;
    let mut zerocount = 0;
    let mut pulse_measurer = PulseMeasurer::new((1_000_000.0 / sampling_rate) as u16);
    loop {
        //let irq_flags_1 = rfm_error!(rfm.read(Registers::IrqFlags1))?;
        let irq_flags_2 = rfm_error!(rfm.read(Registers::IrqFlags2))?;
        //let rssi = rfm_error!(rfm.read(Registers::RssiValue))? as f32 / -2.0;
        /*println!(
            "irq_flags: {:#02x} {:#02x}, RSSI={}",
            irq_flags_1, irq_flags_2, rssi
        );*/
        if irq_flags_2 & 0x20 != 0 {
            let val = rfm_error!(rfm.read(Registers::Fifo))?;
            pulse_measurer.add_bits(val);
            if val == 0 {
                zerocount += 1;
            } else {
                if zerocount > 0 {
                    if zerocount >= 3 {
                        break;
                    }
                    zerocount = 0;
                }
            }
        }
    }
    println!();

    Ok(())
}

struct PulseMeasurer {
    /// Sample duration in microseconds.
    sample_duration: u16,
    /// Whether the current pulse is high or low.
    current_level: bool,
    /// The number of samples in the curren pulse so far.
    pulse_sample_count: u16,
}

impl PulseMeasurer {
    pub fn new(sample_duration: u16) -> Self {
        Self {
            sample_duration,
            current_level: false,
            pulse_sample_count: 0,
        }
    }

    pub fn add_bit(&mut self, bit: bool) {
        if bit == self.current_level {
            self.pulse_sample_count += 1;
        } else {
            self.finish();
            self.current_level = bit;
            self.pulse_sample_count = 1;
        }
    }

    pub fn add_bits(&mut self, byte: u8) {
        for i in 0..8 {
            let bit = byte & (0x80 >> i) != 0;
            self.add_bit(bit);
        }
    }

    pub fn finish(&mut self) {
        if self.pulse_sample_count > 0 {
            println!(
                "{} {} ({} uS)",
                self.pulse_sample_count,
                self.current_level,
                self.pulse_sample_count * self.sample_duration,
            );
        }
        self.pulse_sample_count = 0;
    }
}
