pub mod receiver;
pub mod sender;
pub mod common;

use embassy_stm32::can::{self, filter::ExtendedFilter, BufferedCan, CanConfigurator, RxBuf, TxBuf};
use embedded_can::{ExtendedId};
use heapless::Vec;
use static_cell::StaticCell;

const RODOS_CAN_ID: u8 = 0x1C;

const RX_BUF_SIZE: usize = 200;
const TX_BUF_SIZE: usize = 30;

static RX_BUF: StaticCell<embassy_stm32::can::RxBuf<RX_BUF_SIZE>> = StaticCell::new();
static TX_BUF: StaticCell<embassy_stm32::can::TxBuf<TX_BUF_SIZE>> = StaticCell::new();

/// Constructor and interface to read and write can messages with the RODOS protocol
pub struct RodosCanConfigurator<'d> {
    interface: BufferedCan::<'d, TX_BUF_SIZE, RX_BUF_SIZE>,
}

impl<'d> RodosCanConfigurator<'d> {
    /// create an instance using a base can configurator, a bitrate and a list of topics
    pub fn new(mut can_configurator: CanConfigurator<'d>, bitrate: u32, topics: &[u16]) -> Self {
        // reject all by default
        can_configurator.set_config(
            can::config::FdCanConfig::default()
            .set_global_filter(can::config::GlobalFilter::reject_all())
        );
        // add filters for all relevant topics
        can_configurator.set_bitrate(bitrate);
        let mut filters = topics.into_iter().map(|topic| -> ExtendedFilter {
            let can_id_range_start: u32 = (RODOS_CAN_ID as u32) << (16 + 8) | (*topic as u32) << 8;
            let can_id_range_end: u32 = can_id_range_start | 0xFF;
            ExtendedFilter {
                filter: can::filter::FilterType::Range {
                    from: ExtendedId::new(can_id_range_start).unwrap(),
                    to: ExtendedId::new(can_id_range_end).unwrap()
                },
                action: can::filter::Action::StoreInFifo0,
            }
        }).take(8).collect::<Vec<ExtendedFilter, 8>>();
        // fill up rest of filters with disabled
        while !filters.is_full() {
            filters.push(ExtendedFilter::disable()).unwrap();
        }
        can_configurator.properties().set_extended_filters(&filters.into_array().unwrap());

        // initialize buffered can
        let interface = can_configurator.into_normal_mode()
            .buffered(TX_BUF.init(TxBuf::<TX_BUF_SIZE>::new()), RX_BUF.init(RxBuf::<RX_BUF_SIZE>::new()));

        Self { interface }
    }
    pub fn split(self) -> (receiver::RodosCanReceiver, sender::RodosCanSender) {
        (
            receiver::RodosCanReceiver::new(self.interface.reader()),
            sender::RodosCanSender::new(self.interface.writer())
        )
    }
}
