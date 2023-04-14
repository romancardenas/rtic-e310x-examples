#![no_main]
#![no_std]

use panic_halt as _;
use riscv_rt as _;

#[no_mangle]
#[allow(non_snake_case)]
unsafe fn ClearUART0() {
    // In RISCV-SLIC, we need to define a specific handler
    // for clearing HW interrupts
}

#[rtic::app(device = e310x)]
mod app {
    use slic::Interrupt;
    use hifive1::sprintln;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        // Pends the UART0 interrupt but its handler won't run until *after*
        // `init` returns because interrupts are disabled
        rtic::pend(Interrupt::UART0); // equivalent to NVIC::pend

        sprintln!("init");

        (Shared {}, Local {})
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        // interrupts are enabled again; the `UART0` handler runs at this point

        sprintln!("idle");

        rtic::pend(Interrupt::UART0);

        loop {
            unsafe{rtic::nop()};
        }
    }

    #[task(binds = UART0, local = [times: u32 = 0])]
    fn uart0(cx: uart0::Context) {
        // Safe access to local `static mut` variable
        *cx.local.times += 1;

        sprintln!(
            "UART0 called {} time{}",
            *cx.local.times,
            if *cx.local.times > 1 { "s" } else { "" }
        );
    }
}
