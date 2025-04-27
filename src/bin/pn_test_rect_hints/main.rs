use std::{error::Error, time::Duration};
use dbus::blocking::Connection;

mod proxy;

use proxy::service1::OrgPinenoteService1Ebc1;
use pinenote_dbus_service::kernel::module::rockchip_ebc::{HintBitDepth, HintConvertMode, PixelHints};

trait IntoDBus {
    type Repr;

    fn into_dbus(self) -> Self::Repr;
}

impl IntoDBus for PixelHints {
    type Repr = (u8, u8, bool);

    fn into_dbus(self) -> Self::Repr {
        (self.bit_depth().into(), self.convert_mode().into(), self.redraw())
    }
}

struct HintsRect {
    pub hints: PixelHints,
    pub rect: (i32, i32, i32, i32)
}

impl HintsRect {
    pub fn new(bit_depth: HintBitDepth, convert: HintConvertMode, redraw: bool,
               rect: (i32, i32, i32, i32)) -> Self {

        Self { hints: PixelHints::new(bit_depth, convert, redraw),
            rect
        }
    }
}

impl From<HintsRect> for ((u8, u8, bool), (i32, i32, i32, i32,)) {
    fn from(value: HintsRect) -> Self {
        (value.hints.into_dbus(), value.rect)
    }
}

fn main() -> Result<(), Box<dyn Error>>{
    let c = Connection::new_system()?;

    let proxy = c.with_proxy("org.pinenote.Service1", "/org/pinenote/Service1", Duration::from_millis(5000));
    proxy.set_hints(vec![
        HintsRect::new(
            HintBitDepth::Y1, HintConvertMode::Threshold, true,
            ( 200, 200, 1600, 1200 )
        ).into(),

        HintsRect::new(
            HintBitDepth::Y4, HintConvertMode::Threshold, true,
            ( 300, 300, 1800, 1000 )
        ).into(),

        HintsRect::new(
            HintBitDepth::Y2, HintConvertMode::Dither, true,
            ( 500, 500, 1400, 800 )
        ).into(),
    ])?;

    Ok(())
}
