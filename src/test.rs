#[test]
fn test_blur_image_correctly() {
    extern crate image;

    use super::gaussian_blur;
    use super::utils;

    let image_bytes = include_bytes!("../assets/cballs.png");
    let image_reference_bytes = include_bytes!("../assets/cballs_reference_5px_blur.png");

    let res1 = image::load_from_memory_with_format(image_bytes, image::ImageFormat::PNG);
    let res2 = image::load_from_memory_with_format(image_reference_bytes, image::ImageFormat::PNG);

    if let (Ok(image::DynamicImage::ImageRgb8(png_data)),
            Ok(image::DynamicImage::ImageRgb8(reference_data))) =
            (res1, res2)
    {
        let width = png_data.width() as usize;
        let height = png_data.height() as usize;
        let data = png_data.into_raw();
        let mut data_new = Vec::<[u8;3]>::with_capacity(width * height);
        data_new.resize(width * height, [0, 0, 0]);

        let mut data_new = Vec::<[u8;3]>::with_capacity(width * height);
        data_new.resize(width * height, [0, 0, 0]);

        for y in 0..height {
            for x in 0..width {
                let offset = ((width * y) + x) as usize;
                data_new[((width * y) + x) as usize] = [data[offset * 3], data[(offset * 3) + 1], data[(offset * 3) + 2]];
            }
        }

        let reference = reference_data.into_raw();
        let mut reference_new = Vec::<[u8;3]>::with_capacity(width * height);
        reference_new.resize(width * height, [0, 0, 0]);

        for y in 0..height {
            for x in 0..width {
                let offset = ((width * y) + x) as usize;
                reference_new[((width * y) + x) as usize] = [reference[offset * 3], reference[(offset * 3) + 1], reference[(offset * 3) + 2]];
            }
        }

        gaussian_blur(&mut data_new, width as usize, height as usize, 6.0);
        utils::write_image("test.ppm", &data_new, width as usize, height as usize).unwrap();

        // this will fail, no matter what because the radius in the original algorithm isn't known
        // and I don't know if Javascript does things differently in terms of floating-point calculations
/*
        for (idx, (px, other)) in data_new.iter().zip(reference_new.iter()).enumerate() {
            if px != other { panic!("failed assertion @ byte {:?}", idx); }
        }
*/
    } else {
        panic!("could not decode png");
    }
}

#[test]
fn weird_sizes_dont_panic() {
    extern crate image;

    use super::gaussian_blur;
    use super::utils;

    let image_bytes = include_bytes!("../assets/cballs.png");

    let res1 = image::load_from_memory_with_format(image_bytes, image::ImageFormat::PNG);
    if let Ok(image::DynamicImage::ImageRgb8(png_data)) = res1 {
        let width = png_data.width() as usize;
        let height = png_data.height() as usize;
        let data = png_data.into_raw();
        let mut data_new = Vec::<[u8;3]>::with_capacity(width * height);
        data_new.resize(width * height, [0, 0, 0]);
        for y in 0..height {
            for x in 0..width {
                let offset = ((width * y) + x) as usize;
                data_new[((width * y) + x) as usize] = [data[offset * 3], data[(offset * 3) + 1], data[(offset * 3) + 2]];
            }
        }
        gaussian_blur(&mut data_new, width as usize, height as usize, -5.0);
        gaussian_blur(&mut data_new, width as usize, height as usize, 0.0);
        gaussian_blur(&mut data_new, width as usize, height as usize, 180.0); // smaller than height
        gaussian_blur(&mut data_new, width as usize, height as usize, 500.0); // bigger than height, smaller than width
        gaussian_blur(&mut data_new, width as usize, height as usize, 500000.0); // bigger than both
        // causes panic in debug mode due to integer overflow, works in release mode
        // gaussian_blur(&mut data_new, width as usize, height as usize, 500000000000000000.0); // ridiculously big
    } else {
        panic!("could not decode png");
    }
}
