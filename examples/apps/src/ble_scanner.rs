use bt_hci::cmd::le::LeSetScanParams;
use bt_hci::controller::ControllerCmdSync;
use core::cell::RefCell;
use embassy_futures::join::join;
use embassy_time::{Duration, Timer};
use heapless::Deque;
use trouble_host::prelude::*;

/// Max number of connections
const CONNECTIONS_MAX: usize = 1;
const L2CAP_CHANNELS_MAX: usize = 1;

pub async fn run<C, const L2CAP_MTU: usize>(controller: C)
where
    C: Controller + ControllerCmdSync<LeSetScanParams>,
{
    // Using a fixed "random" address can be useful for testing. In real scenarios, one would
    // use e.g. the MAC 6 byte array as the address (how to get that varies by the platform).
    let mut address: Address = Address::random([0xff, 0x8f, 0x1b, 0x05, 0xe4, 0xff]);

    info!("Our address = {:?}", address);
    let mut resources: HostResources<CONNECTIONS_MAX, L2CAP_CHANNELS_MAX, L2CAP_MTU> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources).set_random_address(address);
    let Host {
        central, mut runner, ..
    } = stack.build();

    let addresses: RefCell<Deque<BdAddr, 128>> = RefCell::new(Deque::new());

    let receiver = runner.receiver();
    let mut scanner = Scanner::new(central);
    let _ = join(
        runner.run_with_handler(async |report| {
            let mut seen = addresses.borrow_mut();
            if seen.iter().find(|b| b.raw() == report.addr.raw()).is_none() {
                info!("discovered: {:?}", report.addr);
                if seen.is_full() {
                    seen.pop_front();
                }
                seen.push_back(report.addr).unwrap();
            }
        }),
        async {
            let mut config = ScanConfig::default();
            config.active = true;
            config.phys = PhySet::M1;
            config.interval = Duration::from_secs(1);
            config.window = Duration::from_secs(1);
            let _session = scanner.scan(&config).await.unwrap();
            // Scan forever
            info!("Starting to scan");
            loop {
                Timer::after(Duration::from_secs(1)).await;
            }
        },
    )
    .await;
}
