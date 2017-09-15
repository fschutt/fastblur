/// Saves an image as a PPM file, for testing / debugging
pub fn write_image<S>(filename: S, data: &[[u8;3]], width: usize, height: usize)
-> Result<(), ::std::io::Error> where S: Into<String>
{
    use std::fs::File;
    use std::io::BufWriter;
    use std::io::Write;

    let mut file = BufWriter::new(File::create(filename.into())?);
    let header = format!("P6\n{}\n{}\n{}\n", width, height, 255);

    file.write(header.as_bytes())?;

    for px in data {
        file.write(px)?;
    }

    Ok(())
}
