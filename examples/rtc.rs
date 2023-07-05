#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]
#![feature(generators)]

use panic_halt as _;
use riscv_rt as _;

#[rtic::app(device = e310x, dispatchers = [SoftLow, SoftHigh])]
mod app {
    use core::{future::Future, task::Poll, pin::Pin, task::Context};

    use hifive1::{hal::prelude::*, sprintln};

    pub async fn yield_now(task: &str) {
    /// Yield implementation
    struct YieldNow {
        yielded: bool,
    }
        sprintln!("[Yield]: {} is yielding", task);

    impl Future for YieldNow {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if self.yielded {
                return Poll::Ready(());
            }

            self.yielded = true;
            cx.waker().wake_by_ref();

            Poll::Pending
        }
    }

    YieldNow { yielded: false }.await
}

    /// HW handler for clearing RTC. When using SLIC, we must
    /// define a ClearX handler for every bypassed HW interrupt
    #[no_mangle]
    #[allow(non_snake_case)]
    unsafe fn ClearRTC() {
        // increase rtccmp to clear HW interrupt
        let rtc = hifive1::hal::DeviceResources::steal().peripherals.RTC;
        let rtccmp = rtc.rtccmp.read().bits();
        sprintln!("\nclear RTC (rtccmp = {})", rtccmp);
        rtc.rtccmp.write(|w| w.bits(rtccmp + 65536));
        // we also pend the lowest priority SW task before the RTC SW task is automatically pended
        //riscv_slic::pend(slic::Interrupt::SoftLow);
        // soft_low::spawn().unwrap();
    }

    #[shared]
    struct Shared {
        counter: u32,
    }

    #[local]
    struct Local {}

    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        // Pends the SoftLow interrupt but its handler won't run until *after*
        // `init` returns because interrupts are disabled
        let dr;
        unsafe {
            dr = hifive1::hal::DeviceResources::steal();
        }
        let pins = dr.pins;
        let p = dr.peripherals;
        let clocks = hifive1::clock::configure(p.PRCI, p.AONCLK, 64.mhz().into());

        // Disable watchdog
        let wdg = p.WDOG;
        wdg.wdogcfg.modify(|_, w| w.enalways().clear_bit());

        hifive1::stdout::configure(
            p.UART0,
            hifive1::pin!(pins, uart0_tx),
            hifive1::pin!(pins, uart0_rx),
            115_200.bps(),
            clocks,
        );
        // sprintln!("init");

        let mut rtc = p.RTC.constrain();
        rtc.disable();
        rtc.set_scale(0);
        rtc.set_rtc(0);
        rtc.set_rtccmp(10000);
        rtc.enable();
        //soft_low::spawn().unwrap(); // TODO: crashes on try_allocate()

        sprintln!("init");
        (Shared { counter: 0 }, Local {})
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        // interrupts are enabled again; the `SoftLow` handler runs at this point
        loop{
            sprintln!("executing idle");
            unsafe {riscv::asm::wfi();} // sleep the microprocessor
        }
    }

    /// HW task executed after receiving an RTC external interrupt
    #[task(binds = RTC, local = [times: u32 = 0], shared = [counter], priority = 2)]
    fn hw_rtc(mut cx: hw_rtc::Context) {
        // Safe access to local `static mut` variable
        sprintln!("    [RTC]: Started");
        cx.shared.counter.lock(|counter| {
            soft_low_1::spawn();
            soft_high::spawn();
            soft_low_2::spawn();

            *counter += 1;
            sprintln!("    [RTC]: Shared: {}", *counter);
        });
        
        *cx.local.times += 1;
        sprintln!(
            "    [RTC]: Local: {}",
            *cx.local.times,
        );


        sprintln!("    [RTC]: Finished");
    }

    /// SW task triggerend during the process of clearing RTC EXTIs
    #[task(local = [times: u32 = 0], shared = [counter], priority = 1)]
    async fn soft_low_1(mut cx: soft_low_1::Context) {
        sprintln!("[SoftLow1]: Started"); 
        // Safe access to local `static mut` variable
        *cx.local.times += 1;
        cx.shared.counter.lock(|counter| {
            *counter += 1;
            sprintln!("[SoftLow1]: Shared: {}", *counter);
        });
        
        yield_now("SoftLow1").await;
        sprintln!(
            "[SoftLow1]: Local: {}",
            *cx.local.times,
        );
        sprintln!("[SoftLow1]: Finished"); 
    }
    
    /// SW task triggerend during the process of clearing RTC EXTIs
    #[task(local = [times: u32 = 0], shared = [counter], priority = 1)]
    async fn soft_low_2(mut cx: soft_low_2::Context) {
        sprintln!("[SoftLow2]: Started"); 
        // Safe access to local `static mut` variable
        *cx.local.times += 1;
        cx.shared.counter.lock(|counter| {
            *counter += 1;
            sprintln!("[SoftLow2]: Shared: {}", *counter);
        });
        yield_now("SoftLow2").await;
        sprintln!(
            "[SoftLow2]: Local: {}",
            *cx.local.times,
        );
        sprintln!("[SoftLow2]: Finished"); 
    }
    
    #[task(local = [times: u32 = 0], shared = [counter], priority = 3)]
    async fn soft_high(mut cx: soft_high::Context) {
        sprintln!("        [SoftHigh]: Started");
        // Safe access to local `static mut` variable
        *cx.local.times += 1;
        
        cx.shared.counter.lock(|counter| {
            *counter += 1;
            sprintln!("        [SoftHigh]: Shared: {}",
                    counter);
        });
        sprintln!("        [SoftHigh]: Local: {}",
            *cx.local.times,
        );
        sprintln!("        [SoftHigh]: Finished");
    }
}
