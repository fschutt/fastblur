extern crate x11_dl;
extern crate fastblur;

fn main() {
    use fastblur::utils;

    let display = x11_get_display();
    let (mut data, width, height) = x11_make_screenshot(display, 0, 0, None, None);
    fastblur::gaussian_blur(&mut data, width as usize, height as usize, 257.0);
    utils::write_image("screenshot.ppm", &data, width as usize, height as usize).unwrap();
}

#[cfg(target_os = "linux")]
fn x11_get_display<'a>() -> &'a mut x11_dl::xlib::Display {

    let xlib = match x11_dl::xlib::Xlib::open() {
        Ok(x) => x,
        Err(xerr) => panic!("Error: {}", xerr.detail()),
    };

    let dpy = unsafe { (xlib.XOpenDisplay)(&0) };

    if dpy.is_null() {
        panic!("Error opening connection to X Server!");
    } else {
        unsafe { &mut *dpy }
    }
}

#[cfg(target_os = "linux")]
fn x11_make_screenshot(display: &mut x11_dl::xlib::Display, offset_x: i32, offset_y: i32, width: Option<i32>, height: Option<i32>)
-> (Vec<[u8; 3]>, u32, u32)
{
    let xlib = match x11_dl::xlib::Xlib::open() {
        Ok(x) => x,
        Err(xerr) => panic!("Error: {}", xerr.detail()),
    };

    let root = unsafe { (xlib.XDefaultRootWindow)(display) };
    let mut gwa: x11_dl::xlib::XWindowAttributes = unsafe { ::std::mem::zeroed() };
    unsafe { (xlib.XGetWindowAttributes)(display, root, &mut gwa) };

    let width = width.unwrap_or(gwa.width);
    let height = height.unwrap_or(gwa.height);

    let image_raw = unsafe { (xlib.XGetImage)(display, root, 0, 0, width as u32, height as u32, (xlib.XAllPlanes)(), x11_dl::xlib::ZPixmap) };

    let image = {
        if image_raw.is_null() {
            panic!("Error getting image!");
        } else {
            unsafe { &mut *image_raw }
        }
    };

    // todo: check if 3 is the correct number
    let capacity = (width * height) as usize;

    let mut screenshot: Vec<[u8; 3]> = Vec::with_capacity(capacity);
    screenshot.resize(capacity, [0, 0, 0]);

    let red_mask = image.red_mask;
    let green_mask = image.green_mask;
    let blue_mask = image.blue_mask;

    for y in offset_y..height {
        for x in offset_x..width {
            let pixel = unsafe { (xlib.XGetPixel)(image,x,y) };

            let blue  = (pixel & blue_mask) as u8;
            let green = ((pixel & green_mask) >> 8) as u8;
            let red   = ((pixel & red_mask)   >> 16) as u8;

            screenshot[((width * y) + x) as usize] = [red, green, blue];
        }
    }

    (screenshot, width as u32, height as u32)
}
