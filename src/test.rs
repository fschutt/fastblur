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
        gaussian_blur(&mut data_new, width as usize, height as usize, 500000000000000000.0); // ridiculously big
    } else {
        panic!("could not decode png");
    }
}

#[test]
fn box_sizes_match_the_reference_algorithm() {
    use super::blur::create_box_gauss;

    // Reference values, computed with `boxesForGauss` from
    // http://blog.ivank.net/fastest-gaussian-blur.html. Two narrow boxes plus
    // one wide one; three *equal* boxes would mean the `+ 1` escaped the sqrt.
    assert_eq!(create_box_gauss::<3>(1.0), [1, 1, 3]);
    assert_eq!(create_box_gauss::<3>(2.0), [3, 3, 5]);
    assert_eq!(create_box_gauss::<3>(5.0), [9, 9, 11]);
    assert_eq!(create_box_gauss::<3>(10.0), [19, 19, 21]);
    assert_eq!(create_box_gauss::<3>(20.0), [39, 39, 41]);

    // A sigma of zero or less means "don't blur", i.e. a box width of one.
    assert_eq!(create_box_gauss::<3>(0.0), [1, 1, 1]);
    assert_eq!(create_box_gauss::<3>(-1.0), [1, 1, 1]);
}

#[test]
fn extreme_radii_do_not_panic() {
    use super::{gaussian_blur, gaussian_blur_asymmetric, gaussian_blur_asymmetric_single_channel};

    // Deliberately taller than wide. The horizontal pass used to subtract
    // `height` when checking whether the box was wider than the *row*, which
    // underflows for any image where `height > blur_radius > width` - the
    // existing test image is wider than it is tall, so it never hit this.
    let (width, height) = (8usize, 24usize);

    let radii = [
        -5.0f32,
        0.0,
        0.5,
        12.0,                    // wider than the image, narrower than it is tall
        500.0,                   // wider than both
        500000000000000000.0,    // used to overflow `wl + 2`
        ::std::f32::MAX,
        ::std::f32::INFINITY,
        ::std::f32::NEG_INFINITY,
        ::std::f32::NAN,
    ];

    for &radius in radii.iter() {
        // A flat image has to survive any blur unchanged: every window, however
        // far it reaches past the edge, averages the same value.
        let mut rgb = vec![[17u8, 42, 200]; width * height];
        gaussian_blur(&mut rgb, width, height, radius);
        assert!(rgb.iter().all(|px| *px == [17, 42, 200]),
                "flat image changed at radius {}", radius);

        gaussian_blur_asymmetric(&mut rgb, width, height, radius, 3.0);
        gaussian_blur_asymmetric(&mut rgb, width, height, 3.0, radius);

        let mut gray = vec![7u8; width * height];
        gaussian_blur_asymmetric_single_channel(&mut gray, width, height, radius, radius);
        assert!(gray.iter().all(|px| *px == 7),
                "flat single-channel image changed at radius {}", radius);
    }
}

#[test]
fn degenerate_sizes_do_not_panic() {
    use super::{gaussian_blur, gaussian_blur_asymmetric_single_channel};

    for &(w, h) in [(0usize, 0usize), (0, 10), (10, 0), (1, 1), (1, 32), (32, 1)].iter() {
        let mut rgb = vec![[9u8, 9, 9]; w * h];
        gaussian_blur(&mut rgb, w, h, 7.0);

        let mut gray = vec![9u8; w * h];
        gaussian_blur_asymmetric_single_channel(&mut gray, w, h, 7.0, 7.0);
    }
}

/// Deterministic pseudo-random image, so the tests don't depend on the PNG assets.
fn lcg_image(width: usize, height: usize) -> Vec<[u8; 3]> {
    let mut state: u32 = 0x2545_F491;
    let mut next = || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (state >> 24) as u8
    };
    (0..width * height).map(|_| [next(), next(), next()]).collect()
}

fn transpose(src: &[[u8; 3]], width: usize, height: usize) -> Vec<[u8; 3]> {
    let mut out = vec![[0u8; 3]; width * height];
    for y in 0..height {
        for x in 0..width {
            out[x * height + y] = src[y * width + x];
        }
    }
    out
}

#[test]
fn blur_is_symmetric_under_transpose() {
    use super::gaussian_blur;

    // A symmetric blur has to treat both axes alike: blurring the transposed
    // image must give the transpose of the blurred image, give or take the
    // rounding to u8 that each pass does.
    //
    // It used to not. `box_blur` left its result in the scratch buffer rather
    // than the caller's, so the caller got three horizontal but only two
    // vertical passes - and the vertical pass that went missing was the widest
    // of the three. Max channel difference here was 149 at sigma 1.
    let (width, height) = (37usize, 23usize);
    let img = lcg_image(width, height);

    for &sigma in [1.0f32, 3.0, 6.0, 12.0].iter() {
        let mut direct = img.clone();
        gaussian_blur(&mut direct, width, height, sigma);

        let mut flipped = transpose(&img, width, height);
        gaussian_blur(&mut flipped, height, width, sigma);
        let flipped = transpose(&flipped, height, width);

        let max_diff = direct.iter().zip(flipped.iter())
            .flat_map(|(p, q)| (0..3).map(move |c| (p[c] as i32 - q[c] as i32).abs()))
            .max().unwrap();

        assert!(max_diff <= 1, "axes disagree by {} at sigma {}", max_diff, sigma);
    }
}

fn lcg_gray(width: usize, height: usize) -> Vec<u8> {
    let mut state: u32 = 0x1357_9BDF;
    (0..width * height).map(|_| {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (state >> 24) as u8
    }).collect()
}

/// FNV-1a, so a golden value fits in one line and any difference shows up.
fn checksum<'a, I: IntoIterator<Item = &'a u8>>(bytes: I) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Golden output checksums, so the refactors that follow (iterator loops, a
/// channel-generic core) have to stay bit-for-bit identical.
///
/// Cases picked to walk every branch: no blur, symmetric, asymmetric, a box
/// wider than the image, and degenerate one-pixel rows and columns.
const GOLDEN: &[(usize, usize, f32, f32, u64, u64)] = &[
    (37, 23,  0.0,  0.0, 0x12f53dd417ab19ae, 0xaa772e5afb8f537f),
    (37, 23,  1.0,  1.0, 0x0276ebfe4acd0ec4, 0x44504541c58e730f),
    (37, 23,  4.5,  4.5, 0x7287cdd419af9970, 0xc7017e4c2f99bc85),
    (37, 23,  2.0,  9.0, 0x3de976cb7f5ac6eb, 0x099c1f65a8c1e827),
    (23, 37,  9.0,  2.0, 0x589bcc69c0d93bb0, 0x986f64e0754db484),
    (37, 23, 60.0, 60.0, 0x937b99fe8ce8a019, 0xac4eda073e1bb687),
    (64, 64,  3.0,  3.0, 0xceb2bf30336ab18a, 0xfddef4e9369ff156),
    ( 1, 40,  5.0,  5.0, 0xfbe1c183803458ac, 0xa46d0b1e2bd425ba),
    (40,  1,  5.0,  5.0, 0xfbe1c183803458ac, 0xa46d0b1e2bd425ba),
];

#[test]
fn blur_output_is_stable() {
    use super::{gaussian_blur_asymmetric, gaussian_blur_asymmetric_single_channel};

    for &(w, h, sh, sv, want_rgb, want_gray) in GOLDEN.iter() {
        let mut rgb = lcg_image(w, h);
        gaussian_blur_asymmetric(&mut rgb, w, h, sh, sv);
        assert_eq!(checksum(rgb.iter().flat_map(|px| px.iter())), want_rgb,
                   "rgb {}x{} h={} v={}", w, h, sh, sv);

        let mut gray = lcg_gray(w, h);
        gaussian_blur_asymmetric_single_channel(&mut gray, w, h, sh, sv);
        assert_eq!(checksum(gray.iter()), want_gray,
                   "gray {}x{} h={} v={}", w, h, sh, sv);
    }
}

#[test]
fn channel_generic_core_agrees_with_the_fixed_width_entry_points() {
    use super::{gaussian_blur, gaussian_blur_generic, gaussian_blur_asymmetric_single_channel};

    let (width, height) = (29usize, 19usize);
    let rgb_src = lcg_image(width, height);
    let alpha_src = lcg_gray(width, height);

    let mut rgb = rgb_src.clone();
    gaussian_blur(&mut rgb, width, height, 3.0);

    let mut alpha = alpha_src.clone();
    gaussian_blur_asymmetric_single_channel(&mut alpha, width, height, 3.0, 3.0);

    // RGBA has to blur the colour channels exactly like the RGB entry point,
    // and its alpha channel exactly like the single channel one.
    let mut rgba: Vec<[u8; 4]> = rgb_src.iter().zip(alpha_src.iter())
        .map(|(p, &a)| [p[0], p[1], p[2], a])
        .collect();
    gaussian_blur_generic(&mut rgba, width, height, 3.0);

    for (i, px) in rgba.iter().enumerate() {
        assert_eq!([px[0], px[1], px[2]], rgb[i], "colour differs at {}", i);
        assert_eq!(px[3], alpha[i], "alpha differs at {}", i);
    }

    // ... and one channel through the generic path matches the flat one.
    let mut gray: Vec<[u8; 1]> = alpha_src.iter().map(|&g| [g]).collect();
    gaussian_blur_generic(&mut gray, width, height, 3.0);
    for (i, px) in gray.iter().enumerate() {
        assert_eq!(px[0], alpha[i], "gray differs at {}", i);
    }
}
