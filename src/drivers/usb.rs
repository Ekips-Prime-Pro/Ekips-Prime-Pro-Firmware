//! USB CDC-ACM serial console.
//!
//! - USB OTG FS on PA11/PA12 with the internal PHY, VBUS sensing on PA9
//!   (\[PB\] `platform.c` L963-1015).
//! - VID:PID `1209:0001` (pid.codes test ID, decision D-008). This must be
//!   replaced by an own PID before any release.
//! - Everything received is echoed back. Text queued with [`write`] (e.g.
//!   from the VM's `print`) is sent as well.
//!
//! **Untested on hardware.**

use embassy_futures::select::{Either, select};
use embassy_stm32::peripherals::{PA11, PA12, USB_OTG_FS};
use embassy_stm32::usb::{self, Driver};
use embassy_stm32::{Peri, bind_interrupts, peripherals};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pipe::Pipe;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;
use embassy_usb::{Builder, UsbDevice};
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    OTG_FS => usb::InterruptHandler<peripherals::USB_OTG_FS>;
});

/// pid.codes test VID (decision D-008).
pub const USB_VID: u16 = 0x1209;
/// pid.codes test PID (decision D-008).
pub const USB_PID: u16 = 0x0001;
const MAX_PACKET: u16 = 64;

type UsbDriver = Driver<'static, USB_OTG_FS>;

/// Outgoing text, filled by [`write`] and drained by the USB task.
static TX: Pipe<CriticalSectionRawMutex, 512> = Pipe::new();

/// Queues bytes for the host. Drops what does not fit (never blocks), so a
/// disconnected console cannot stall the program.
pub fn write(data: &[u8]) {
    let _ = TX.try_write(data);
}

/// Pins and peripheral owned by the USB stack.
pub struct UsbResources {
    /// USB OTG FS.
    pub otg: Peri<'static, USB_OTG_FS>,
    /// PA12, D+.
    pub dp: Peri<'static, PA12>,
    /// PA11, D-.
    pub dm: Peri<'static, PA11>,
}

/// Builds the USB device and starts its tasks.
pub fn start(spawner: embassy_executor::Spawner, r: UsbResources) {
    static EP_OUT: StaticCell<[u8; 256]> = StaticCell::new();
    static CONFIG_DESC: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESC: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
    static STATE: StaticCell<State<'static>> = StaticCell::new();

    let mut config = usb::Config::default();
    // VBUS is wired to PA9 (Pybricks reads it as a GPIO); the OTG core can
    // sense it directly. The hub is self-powered (battery), so VBUS
    // detection is required by the USB spec.
    config.vbus_detection = true;
    let driver = Driver::new_fs(r.otg, Irqs, r.dp, r.dm, EP_OUT.init([0; 256]), config);

    let mut usb_config = embassy_usb::Config::new(USB_VID, USB_PID);
    usb_config.manufacturer = Some("Ekips Prime Pro");
    usb_config.product = Some("Ekips Prime Pro Hub");
    usb_config.serial_number = None;
    usb_config.max_power = 100;
    usb_config.max_packet_size_0 = 64;
    usb_config.self_powered = true;

    let mut builder = Builder::new(
        driver,
        usb_config,
        CONFIG_DESC.init([0; 256]),
        BOS_DESC.init([0; 256]),
        &mut [],
        CONTROL_BUF.init([0; 64]),
    );
    let class = CdcAcmClass::new(&mut builder, STATE.init(State::new()), MAX_PACKET);
    let device = builder.build();

    spawner.spawn(usb_task(device).unwrap());
    spawner.spawn(console_task(class).unwrap());
}

#[embassy_executor::task]
async fn usb_task(mut device: UsbDevice<'static, UsbDriver>) {
    device.run().await;
}

#[embassy_executor::task]
async fn console_task(mut class: CdcAcmClass<'static, UsbDriver>) {
    loop {
        class.wait_connection().await;
        defmt::info!("USB console connected");
        let _ = class
            .write_packet(b"Ekips Prime Pro firmware (untested build)\r\n")
            .await;
        match serve(&mut class).await {
            Ok(()) | Err(EndpointError::Disabled) => defmt::info!("USB console disconnected"),
            Err(EndpointError::BufferOverflow) => defmt::warn!("USB buffer overflow"),
        }
    }
}

async fn serve(class: &mut CdcAcmClass<'static, UsbDriver>) -> Result<(), EndpointError> {
    let mut rx = [0u8; MAX_PACKET as usize];
    let mut tx = [0u8; MAX_PACKET as usize];
    loop {
        match select(class.read_packet(&mut rx), TX.read(&mut tx)).await {
            Either::First(n) => {
                let n = n?;
                // Echo.
                class.write_packet(&rx[..n]).await?;
            }
            Either::Second(n) => class.write_packet(&tx[..n]).await?,
        }
    }
}
