use wasm_bindgen::{prelude::wasm_bindgen, Clamped, JsValue};
use web_sys::{CanvasRenderingContext2d, ImageData};

mod drawing;

#[wasm_bindgen]
pub fn draw(ctx: &CanvasRenderingContext2d, width: u32, height: u32) -> Result<(), JsValue> {
    // The real workhorse of this algorithm, generating pixel data
    let mut data = vec![0u8; (width * height) as usize * 4];
    let pic: &mut [u32] = unsafe { std::mem::transmute(data.as_mut_slice()) };

    let cw = 10;
    let ch = 10;

    drawing::draw_bg(pic, width / cw, height / ch, cw, ch, width / cw);

    let data = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&data), width, height)?;
    ctx.put_image_data(&data, 0.0, 0.0)
}
