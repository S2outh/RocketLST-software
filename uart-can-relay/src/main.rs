#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::{
    bind_interrupts,
    can::{self, CanConfigurator, CanRx},
    gpio::{Level, Output, Speed},
    mode::Async,
    peripherals::*,
    usart::{self, Uart, UartTx}
};
use embedded_can::Id;
use heapless::Vec;
use embassy_time::Timer;
use embedded_io_async::Write;
use {defmt_rtt as _, panic_probe as _};

// bin can interrupts
bind_interrupts!(struct Irqs {
    TIM16_FDCAN_IT0 => can::IT0InterruptHandler<FDCAN1>;
    TIM17_FDCAN_IT1 => can::IT1InterruptHandler<FDCAN1>;
    USART3_4_5_6_LPUART1 => usart::InterruptHandler<USART6>;
});

const CAN_ID: u16 = 0x00;

#[embassy_executor::task]
async fn sender(mut can: CanRx<'static>, mut uart: UartTx<'static, Async>) {

    let mut seq_num: u16 = 0;
    loop {
        match can.read().await {
            Ok(envelope) => {
                if let Id::Standard(id) = envelope.frame.header().id() {
                    if id.as_raw() != CAN_ID {
                        continue;
                    }
                }

                let header = [
                    0x22, 0x69, // Uart start bytes
                    envelope.frame.data().len() as u8 + 6, // packet length
                    0x00, 0x01, // Hardware ID
                    (seq_num >> 8) as u8, seq_num as u8, // SeqNum
                    0x11 // Destination
                ];
                seq_num = seq_num.wrapping_add(1);

                let mut packet: Vec<u8, 254> = Vec::new();
                packet.extend_from_slice(&header).unwrap();
                packet.extend_from_slice(envelope.frame.data()).unwrap();

                if let Err(e) = uart.write_all(envelope.frame.data()).await {
                    error!("dropped frames: {}", e)
                }
            }
            Err(_) => error!("error in frame!"),
        };

        Timer::after_millis(250).await;
    }
}

// async fn receiver(mut can: CanTx<'static>, mut uart: UartRx<'static, Async>) {
//     // TODO
//     loop {
//         let frame = Frame::new_standard(0x321, &[0xBE, 0xEF, 0xDE, 0xAD]).unwrap(); // test data to be send
//         info!("writing frame");
//         can.write(&frame).await;
//     }
// }

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Launching");

    // -- CAN configuration
    let mut can_config = CanConfigurator::new(p.FDCAN1, p.PA11, p.PA12, Irqs);

    can_config.set_bitrate(500_000); //to be ajusted

    // set standby pin to low
    let _can_standby = Output::new(p.PA10, Level::Low, Speed::Low);

    let (_can_tx, can_rx, _can_p) = can_config.into_normal_mode().split();

    // -- Uart configuration
    let mut config = usart::Config::default();
    config.baudrate = 115200;
    let (uart_tx, _uart_rx) = Uart::new_with_rtscts(p.USART6,
        p.PA5, p.PA4,
        Irqs,
        p.PA7, p.PA6,
        p.DMA1_CH1, p.DMA1_CH2,
        config).unwrap().split();

    spawner
        .spawn(sender(can_rx, uart_tx))
        .unwrap();

    let mut led = Output::new(p.PA2, Level::High, Speed::Low);

    loop {
        led.set_high();
        Timer::after_millis(1000).await;

        led.set_low();
        Timer::after_millis(1000).await;
    }
}
