mod whisper_timestamped;
mod whisper;

pub use whisper_timestamped::{WhisperTsJson, WhisperTsSegment, WhisperTsWord};
pub use whisper::{WhisperJson, WhisperSegment};