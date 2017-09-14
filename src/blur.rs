pub fn gaussian_blur(data: &mut Vec<[u8;3]>, width: usize, height: usize, blur_radius: f32)
{
    let mut backbuf = data.clone();
    let bxs = create_box_gauss(blur_radius, 3);

    box_blur(&mut backbuf, data, width, height, ((bxs[0] - 1) / 2) as usize);
    box_blur(&mut backbuf, data, width, height, ((bxs[1] - 1) / 2) as usize);
    box_blur(&mut backbuf, data, width, height, ((bxs[2] - 1) / 2) as usize);
}

#[inline]
fn create_box_gauss(sigma: f32, n: usize)
-> Vec<i32>
{
    let n_float = n as f32;

    // Ideal averaging filter width
    let w_ideal = (12.0 * sigma * sigma / n_float).sqrt() + 1.0;
    let mut wl: i32 = w_ideal.floor() as i32;

    if wl % 2 == 0 { wl -= 1; };

    let wu = wl + 2;

    let wl_float = wl as f32;
    let m_ideal = (12.0 * sigma * sigma - n_float * wl_float * wl_float - 4.0 * n_float * wl_float - 3.0 * n_float) /
                  (-4.0 * wl_float - 4.0);
    let m: usize = m_ideal.round() as usize;

    let mut sizes = Vec::<i32>::new();

    for i in 0..n {
        if i < m {
            sizes.push(wl);
        } else {
            sizes.push(wu);
        }
    }

    sizes
}

/// Needs 2x the same image
#[inline]
fn box_blur(backbuf: &mut Vec<[u8;3]>, frontbuf: &mut Vec<[u8;3]>, width: usize, height: usize, blur_radius: usize)
{
    box_blur_vert(backbuf, frontbuf, width, height, blur_radius);
    box_blur_horz(backbuf, frontbuf, width, height, blur_radius);
}

#[inline]
fn box_blur_vert(backbuf: &mut Vec<[u8;3]>, frontbuf: &mut Vec<[u8;3]>, width: usize, height: usize, blur_radius: usize)
{
    // todo
}

#[inline]
fn box_blur_horz(backbuf: &mut Vec<[u8;3]>, frontbuf: &mut Vec<[u8;3]>, width: usize, height: usize, blur_radius: usize)
{
    /*

    // Original: http://blog.ivank.net/fastest-gaussian-blur.html

    function boxBlurH_4(scl, tcl, w, h, r) {
        var iarr = 1 / (r + r + 1);
        for (var i = 0; i < h; i++) {

            var ti = i * w,
                li = ti,
                ri = ti + r;

            var fv = scl[ti],
                lv = scl[ti + w - 1],
                val = (r + 1) * fv;

            for (var j = 0; j < r; j++) {
                val += scl[ti + j];
            }

            for (var j = 0; j <= r; j++) {
                val += scl[ri++] - fv;
                tcl[ti++] = Math.round(val * iarr);
            }

            for (var j = r + 1; j < w - r; j++) {
                val += scl[ri++] - scl[li++];
                tcl[ti++] = Math.round(val * iarr);
            }

            for (var j = w - r; j < w; j++) {
                val += lv - scl[li++];
                tcl[ti++] = Math.round(val * iarr);
            }
        }
    }

    */

    let iarr = 1.0 / (blur_radius + blur_radius + 1) as f32;

    for i in 0..height {

        let mut ti: usize = i * width;
        let mut li: usize = ti;
        let mut ri: usize = ti + blur_radius;

        let fv: [u8;3] = backbuf[ti];
        let lv: [u8;3] = backbuf[ti + width - 1];

        let mut val_r: isize = (blur_radius as isize + 1) * (fv[0] as isize);
        let mut val_g: isize = (blur_radius as isize + 1) * (fv[1] as isize);
        let mut val_b: isize = (blur_radius as isize + 1) * (fv[2] as isize);

        for j in 0..blur_radius {
            let bb = backbuf[ti + j];
            val_r += bb[0] as isize;
            val_g += bb[1] as isize;
            val_b += bb[2] as isize;
        }

        for j in 0..(blur_radius + 1) {
            let bb = backbuf[ri + 1]; ri += 1;
            val_r += bb[0] as isize - fv[0] as isize;
            val_g += bb[1] as isize - fv[1] as isize;
            val_b += bb[2] as isize - fv[2] as isize;

            frontbuf[ti + 1] = [(val_r as f32 * iarr).round() as u8,
                                (val_g as f32 * iarr).round() as u8,
                                (val_b as f32 * iarr).round() as u8];
            ti += 1;
        }

        for j in (blur_radius + 1)..(width - blur_radius) { // <- it works with (width - blur_radius - 1),
                                                            // but there are a few pixel errors?

            let bb1 = backbuf[ri + 1]; ri += 1; // <-- one off error, but how?
            let bb2 = backbuf[li + 1]; li += 1;

            val_r += bb1[0] as isize - bb2[0] as isize;
            val_g += bb1[1] as isize - bb2[1] as isize;
            val_b += bb1[2] as isize - bb2[2] as isize;

            frontbuf[ti + 1] = [(val_r as f32 * iarr).round() as u8,
                                (val_g as f32 * iarr).round() as u8,
                                (val_b as f32 * iarr).round() as u8];
            ti += 1;
        }

        for j in (width - blur_radius)..width {
            let bb = backbuf[li + 1]; li += 1;

            val_r += lv[0] as isize - bb[0] as isize;
            val_g += lv[1] as isize - bb[1] as isize;
            val_b += lv[2] as isize - bb[2] as isize;

            frontbuf[ti + 1] = [(val_r as f32 * iarr).round() as u8,
                                (val_g as f32 * iarr).round() as u8,
                                (val_b as f32 * iarr).round() as u8];
            ti += 1;
        }
    }

}

/*

inline void box_blur_horz(unsigned char * backbuf, unsigned char* frontbuf, int width, int height, unsigned float blur_radius) {

}

inline void box_blur_vert(unsigned char * backbuf, unsigned char* frontbuf, int width, int height, unsigned float blur_radius) {

    float iarr = 1.0f / (float)(blur_radius + blur_radius + 1);

    for(int i = 0; i < height; i++) {

        // for each color component (r, g and b)
        for(int col_i = 0; col_i < 3; col_i++) {

            long ti = (i + col_i) * width;
            long li = ti;
            unsigned float ri = ti + blur_radius;

            unsigned float fv = (unsigned int)backbuf[ti];
            unsigned float lv = (unsigned int)backbuf[ti+w-1];
            unsigned float lv = (unsigned int)backbuf[ti+w-1];
            unsigned float val = (blur_radius + 1.0f)* fv;

            for(var j=0; j<r; j++) {
                val += (unsigned float)backbuf[ti+j];
            }

            for(var j=0  ; j<=r ; j++) {
                val += (unsigned float)backbuf[ri++] - fv;
                frontbuf[ti++] = (unsigned char)round(val * iarr);
            }

            for(var j=r+1; j<w-r; j++) {
                val += (unsigned float)backbuf[ri++] - backbuf[li++];
                frontbuf[ti++] = (unsigned char)round(val * iarr);
            }

            for(var j=w-r; j<w  ; j++) {
                val += lv - (unsigned float)backbuf[li++];
                frontbuf[ti++] = (unsigned char)round(val * iarr);
            }
        }
    }
}

inline void box_blur(unsigned char * backbuf, unsigned char* frontbuf, int width, int height, unsigned float blur_radius) {
    box_blur_horz(backbuf, frontbuf, width, height, blur_radius);
    box_blur_vert(backbuf, frontbuf, width, height, blur_radius);
}

/*
function boxBlurH_4(scl, tcl, w, h, r) {
    var iarr = 1 / (r+r+1);
    for(var i=0; i<h; i++) {
        var ti = i*w, li = ti, ri = ti+r;
        var fv = scl[ti], lv = scl[ti+w-1], val = (r+1)*fv;
        for(var j=0; j<r; j++) val += scl[ti+j];
        for(var j=0  ; j<=r ; j++) { val += scl[ri++] - fv       ;   tcl[ti++] = Math.round(val*iarr); }
        for(var j=r+1; j<w-r; j++) { val += scl[ri++] - scl[li++];   tcl[ti++] = Math.round(val*iarr); }
        for(var j=w-r; j<w  ; j++) { val += lv        - scl[li++];   tcl[ti++] = Math.round(val*iarr); }
    }
}
function boxBlurT_4 (scl, tcl, w, h, r) {
    var iarr = 1 / (r+r+1);
    for(var i=0; i<w; i++) {
        var ti = i, li = ti, ri = ti+r*w;
        var fv = scl[ti], lv = scl[ti+w*(h-1)], val = (r+1)*fv;
        for(var j=0; j<r; j++) val += scl[ti+j*w];
        for(var j=0  ; j<=r ; j++) { val += scl[ri] - fv     ;  tcl[ti] = Math.round(val*iarr);  ri+=w; ti+=w; }
        for(var j=r+1; j<h-r; j++) { val += scl[ri] - scl[li];  tcl[ti] = Math.round(val*iarr);  li+=w; ri+=w; ti+=w; }
        for(var j=h-r; j<h  ; j++) { val += lv      - scl[li];  tcl[ti] = Math.round(val*iarr);  li+=w; ti+=w; }
    }
}
*/

void gaussian_blur_fast(unsigned char * backbuf, unsigned char * frontbuf, int width, int height, unsigned float blur_radius) {
    std::vector<int> bxs = create_box_gauss(blur_radius, 3.0f);

    for (int data: bxs) { printf("%d\n", data); }

    box_blur(backbuf, frontbuf, width, height, (float)(bxs[0]-1) / 2.0f);
    box_blur(backbuf, frontbuf, width, height, (float)(bxs[1]-1) / 2.0f);
    box_blur(backbuf, frontbuf, width, height, (float)(bxs[2]-1) / 2.0f);
}
*/
