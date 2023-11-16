use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, WriteColor, BufferWriter};

pub struct ColorOutput;

impl ColorOutput {
    fn write_color(message: &str, color: Color) -> io::Result<()> {
        let mut bufwtr = BufferWriter::stderr(ColorChoice::Always);
        let mut buffer = bufwtr.buffer();
        buffer.set_color(ColorSpec::new().set_fg(Some(color)))?;
        // writeln!(&mut buffer, message)?;
        writeln!(&mut buffer, "message")?;
        bufwtr.print(&buffer)
    }
}