#![no_main]
#![no_std]

use panic_halt as _;
use riscv_rt as _;

#[rtic::app(device = e310x, dispatchers = [SoftLow])]
mod app {
    use hifive1::sprintln;
    use slic::Interrupt;

    /// HW handler for clearing RTC. When using SLIC, we must
    /// define a ClearX handler for every bypassed HW interrupt
    #[no_mangle]
    #[allow(non_snake_case)]
    unsafe fn ClearRTC() {
        // increase rtccmp to clear HW interrupt
        let rtc = hifive1::hal::DeviceResources::steal().peripherals.RTC;
        let rtccmp = rtc.rtccmp.read().bits();
        sprintln!("clear RTC (rtccmp = {})", rtccmp);
        rtc.rtccmp.write(|w| w.bits(rtccmp + 65536));
        // we also pend the lowest priority SW task before the RTC SW task is automatically pended
        riscv_slic::pend(Interrupt::SoftLow);
    }

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        // Pends the SoftLow interrupt but its handler won't run until *after*
        // `init` returns because interrupts are disabled
        rtic::pend(Interrupt::SoftLow);
        sprintln!("init");
        (Shared {}, Local {})
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        // interrupts are enabled again; the `SoftLow` handler runs at this point
        sprintln!("idle");
        loop {
            unsafe { rtic::nop() };
        }
    }

    /// HW task executed after receiving an RTC external interrupt
    #[task(binds = RTC, local = [times: u32 = 0], priority = 2)]
    fn hw_rtc(cx: hw_rtc::Context) {
        // Safe access to local `static mut` variable
        *cx.local.times += 1;

        sprintln!(
            "hw_rtc called {} time{}",
            *cx.local.times,
            if *cx.local.times > 1 { "s" } else { "" }
        );
    }

    /// SW task triggerend during the process of clearing RTC EXTIs
    #[task(local = [times: u32 = 0], priority = 1)]
    async fn soft_low(cx: soft_low::Context) {
        // Safe access to local `static mut` variable
        *cx.local.times += 1;

        sprintln!(
            "soft_low called {} time{}",
            *cx.local.times,
            if *cx.local.times > 1 { "s" } else { "" }
        );
    }
}
