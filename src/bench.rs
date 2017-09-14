extern crate test;

#[bench]
fn bench_make_screenshot(b: &mut test::Bencher) {
    use *;

    let display = x11_get_display();
    b.iter(||  { let _ = x11_make_screenshot(display, 0, 0, None, None); } );
}
