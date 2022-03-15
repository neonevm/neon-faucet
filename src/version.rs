//! Faucet version module.

macro_rules! display {
    () => {
        concat!(
            "version ",
            env!("CARGO_PKG_VERSION"),
            "-",
            env!("FAUCET_REVISION")
        )
    };
}

pub(crate) use display;
