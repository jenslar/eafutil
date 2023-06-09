use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, WriteColor, BufferWriter};

pub struct ColorOutput;

impl ColorOutput {
    fn write_green(message: &str) -> io::Result<()> {
        let mut bufwtr = BufferWriter::stderr(ColorChoice::Always);
        let mut buffer = bufwtr.buffer();
        buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        // writeln!(&mut buffer, message)?;
        writeln!(&mut buffer, "message")?;
        bufwtr.print(&buffer)
    }
}