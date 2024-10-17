//! Data structures corresponding to Whisper JSON-files.
//! See: <https://github.com/openai/whisper>

use std::{
    fs::{read_to_string, File}, io::Write, path::{Path, PathBuf}
};

use eaf_rs::{eaf::annotation::AnnotationBuilder, Annotation, Eaf, EafError, LinguisticType, StereoType, Tier, TimeOrder, TimeSlot};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json;

use crate::clips::Clips;

/// Whisper result JSON structure.
/// See: <https://github.com/openai/whisper>
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WhisperJson {
    /// Detected language, e.g. "en" for English.
    /// Seems optional for some Whisper derivations (e.g. Whisper-Faster)
    language: Option<String>,
    /// Segmented text.
    segments: Vec<WhisperSegment>,
    /// Full text.
    text: Option<String>,
}

impl WhisperJson {
    /// Reads a Whisper transcription in JSON-form.
    /// Supports word level timestamps if present.
    pub fn read(path: &Path) -> std::io::Result<Self> {
        let string = read_to_string(path)?;
        Ok(serde_json::from_str::<Self>(&string)?)
    }

    /// Writes to disk as JSON.
    pub fn write(&self, path: &Path) -> std::io::Result<usize> {
        let string = serde_json::to_string(self)?;
        let mut file = File::create(path)?;
        file.write(string.as_bytes())
    }

    pub fn from_paths(paths: &[PathBuf], clips: &Clips) -> std::io::Result<Self> {
        if paths.is_empty() {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "No JSON-paths provived"))
        }

        let mut json = paths.iter()
            .map(|p| {
                let ts_ms = clips.get_timestamps(p)
                    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Failed to extract timestamps"))?;
                Ok(Self::read(p)?.offset(ts_ms.0 as f64 / 1000.))
            })
            .collect::<std::io::Result<Vec<Self>>>()?;

        if paths.len() > 1 {
            return json[0].to_owned().join(&mut json[1..])
        } else {
            return Ok(json[0].to_owned())
        }
    }

    /// Offsets all time values by the specified offset.
    pub fn offset(self, offset: f64) -> Self {
        Self {
            segments: self.segments.into_iter().map(|s| s.offset(offset)).collect(),
            ..self
        }
    }

    /// Join multiple Whisper timestamped struct.
    /// Language will reuse that of `self`.
    /// Assumed that timestamps are correct and adjusted.
    pub fn join(&self, others: &mut [Self]) -> std::io::Result<Self> {
        let mut text: Vec<&str> = Vec::new();
        let mut segments: Vec<WhisperSegment> = self.segments.to_vec();
        if let Some(self_txt) = self.text.as_deref() {
            text.push(&self_txt);
            for w in others.iter_mut() {
                if let Some(txt) = w.text.as_deref() {
                    text.push(txt);
                }
                // let ts = w.last_end()
                segments.append(&mut w.segments);
            }
        }

        Ok(Self {
            text: Some(text.join("")),
            segments,
            ..self.to_owned()
        })
    }

    /// Filter out segments that may no be speech.
    /// Any segment with a `no_speech_prob` value
    /// above `threshold` will be discarded.
    pub fn filter_no_speech(self, threshold: f64) -> Self {
        let filtered_segments: Vec<_> = self.segments.into_iter()
            .filter(|s| s.no_speech_prob < threshold)
            .collect();
        let text = filtered_segments.iter().map(|s| s.text.as_str()).join("");
        Self {
            segments: filtered_segments,
            text: Some(text),
            ..self
        }
    }

    /// Generates EAF with three tiers:
    /// - `segments`: full speech segment
    /// - `words`: aligned word tokens (within timespan of corresponding segment)
    ///  `confidence`: confidence of speech segment
    pub fn to_eaf(&self) -> Result<Eaf, EafError> {
        let mut eaf = Eaf::default();

        let segments_id = "segments";
        let words_id = "words";
        let avg_logprob_id = "avg_logprob";
        let compression_ratio_id = "compression_ratio";
        let whisper_id_id = "id";
        let no_speech_prob_id = "no_speech_prob";
        let seek_id = "seek";
        let temperature_id = "temperature";

        // Main tier annotations
        let mut main_annotations: Vec<Annotation> = Vec::new();
        // Word level annotations, time-aligned
        let mut ref_annotations_words: Vec<Annotation> = Vec::new();
        // Whisper reference values symbolic association
        let mut ref_annotations_avgprob: Vec<Annotation> = Vec::new();
        let mut ref_annotations_compression_ratio: Vec<Annotation> = Vec::new(); // 11.868421052631579,
        let mut ref_annotations_id: Vec<Annotation> = Vec::new(); // 227,
        let mut ref_annotations_no_speech_prob: Vec<Annotation> = Vec::new(); // 0.6319243311882019,
        let mut ref_annotations_seek: Vec<Annotation> = Vec::new(); // 70004,
        let mut ref_annotations_temperature: Vec<Annotation> = Vec::new(); // 0.0,

        let mut timeslots: Vec<TimeSlot> = Vec::new();

        for segment in self.segments.iter() {
            let base_annot_idx = 1
            + main_annotations.len()
            + ref_annotations_words.len()
            + ref_annotations_avgprob.len()
            + ref_annotations_compression_ratio.len()
            + ref_annotations_id.len()
            + ref_annotations_no_speech_prob.len()
            + ref_annotations_seek.len()
            + ref_annotations_temperature.len();
            let base_ts_idx = 1 + timeslots.len();

            // Generate timeslots
            // Generate main annotation
            main_annotations.push(segment.to_alignable(base_annot_idx, base_ts_idx)?);
            // Add main annotation timeslots
            timeslots.extend(segment.to_timeslots(base_ts_idx));

            
            // Generate ref annotations with words, alignable annotations (within parent tier annotation timespans)
            let (words, words_ts) = segment.words_to_alignable(base_annot_idx + 1, base_ts_idx + 2)?;
            let a_id_conf_index_base = base_annot_idx + words.len();
            
            ref_annotations_words.extend(words);

            timeslots.extend(words_ts);
            
            // Generate ref_annotations for confidence (no time slots)
            ref_annotations_avgprob.push(segment.to_referred(a_id_conf_index_base + 1, base_annot_idx, WhisperFieldType::AvgLogProb)?);
            ref_annotations_compression_ratio.push(segment.to_referred(a_id_conf_index_base + 2, base_annot_idx, WhisperFieldType::CompressionRatio)?);
            ref_annotations_id.push(segment.to_referred(a_id_conf_index_base + 3, base_annot_idx, WhisperFieldType::Id)?);
            ref_annotations_no_speech_prob.push(segment.to_referred(a_id_conf_index_base + 4, base_annot_idx, WhisperFieldType::NoSpeechProb)?);
            ref_annotations_seek.push(segment.to_referred(a_id_conf_index_base + 5, base_annot_idx, WhisperFieldType::Seek)?);
            ref_annotations_temperature.push(segment.to_referred(a_id_conf_index_base + 6, base_annot_idx, WhisperFieldType::Temperature)?);
        }

        let lingtype_words = LinguisticType::new(words_id, Some(&StereoType::IncludedIn));
        let lingtype_ref_id = "whisper_ref_values";
        let lingtype_ref = LinguisticType::new(lingtype_ref_id, Some(&StereoType::SymbolicAssociation));
        
        let main_tier = Tier::new(
            segments_id,
            Some(&main_annotations),
            None,
            None
        );
        let ref_tier_words = Tier::new(
            words_id,
            Some(&ref_annotations_words),
            Some(words_id),
            Some(segments_id)
        );
        let ref_tier_avg_logprob = Tier::new(
            avg_logprob_id,
            Some(&ref_annotations_avgprob),
            Some(lingtype_ref_id),
            Some(segments_id)
        );
        let ref_tier_compression_ratio = Tier::new(
            compression_ratio_id,
            Some(&ref_annotations_compression_ratio),
            Some(lingtype_ref_id),
            Some(segments_id)
        );
        let ref_tier_id = Tier::new(
            whisper_id_id,
            Some(&ref_annotations_id),
            Some(lingtype_ref_id),
            Some(segments_id)
        );
        let ref_tier_no_speech_prob = Tier::new(
            no_speech_prob_id,
            Some(&ref_annotations_no_speech_prob),
            Some(lingtype_ref_id),
            Some(segments_id)
        );
        let ref_tier_seek = Tier::new(
            seek_id,
            Some(&ref_annotations_seek),
            Some(lingtype_ref_id),
            Some(segments_id)
        );
        let ref_tier_temperature = Tier::new(
            temperature_id,
            Some(&ref_annotations_temperature),
            Some(lingtype_ref_id),
            Some(segments_id)
        );

        eaf.time_order = TimeOrder { time_slots: timeslots };
        eaf.tiers.push(main_tier);
        eaf.tiers.push(ref_tier_words);
        eaf.tiers.push(ref_tier_avg_logprob);
        eaf.tiers.push(ref_tier_compression_ratio);
        eaf.tiers.push(ref_tier_id);
        eaf.tiers.push(ref_tier_no_speech_prob);
        eaf.tiers.push(ref_tier_seek);
        eaf.tiers.push(ref_tier_temperature);
        eaf.add_linguistic_type(&lingtype_words, true);
        eaf.add_linguistic_type(&lingtype_ref, true);

        Ok(eaf)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WhisperSegment {
    avg_logprob: f64, // -0.5485809681027435,
    compression_ratio: f64, // 1.614213197969543,
    /// Segment end in seconds.
    end: f64, // 4.84,
    /// Segment ID.
    id: usize, // 0,
    no_speech_prob: f64, // 0.2937859296798706,
    seek: usize, // 0,
    /// Segment start in seconds.
    start: f64, // 0.0,
    temperature: f64, // 0.0,
    /// Full segment text.
    text: String, // " Ah, no, no, no. Wait, wait, wait, wait. There are some other cars.",
    /// Tokens.
    tokens: Vec<usize>, // [ 50364, 2438, 11, 572, 11, 572, 11, 572, 13, ...]
    /// Words.
    /// Only available if word level segmentation is set
    /// when running Whisper.
    words: Vec<WhisperWord>,
}

pub enum WhisperFieldType {
    // "avg_logprob": -0.11704471164279513,
    AvgLogProb,
    // "compression_ratio": 11.868421052631579,
    CompressionRatio,
    // "end": 706.14,
    End,
    // "id": 227,
    Id,
    // "no_speech_prob": 0.6319243311882019,
    NoSpeechProb,
    // "seek": 70004,
    Seek,
    // "start": 705.02,
    Start,
    // "temperature": 0.0,
    Temperature,
    // "text": " Tänk om det är en P3.",
    Text,
    // "tokens":
    Tokens,
}

impl WhisperFieldType {
    pub fn to_string(&self, segment: &WhisperSegment) -> String {
        match self {
            Self::AvgLogProb => segment.avg_logprob.to_string(),
            Self::CompressionRatio => segment.compression_ratio.to_string(),
            Self::End => segment.end.to_string(),
            Self::Id => segment.id.to_string(),
            Self::NoSpeechProb => segment.no_speech_prob.to_string(),
            Self::Seek => segment.seek.to_string(),
            Self::Start => segment.start.to_string(),
            Self::Temperature => segment.temperature.to_string(),
            Self::Text => segment.text.to_owned(),
            Self::Tokens => segment.tokens.iter().map(|n| n.to_string()).join(", "),
        }
    }
}

impl WhisperSegment {
    /// Returns an aligned ELAN annotation.
    pub fn to_alignable(
        &self,
        annotation_index: usize,
        timeslot_index: usize
    ) -> Result<Annotation, EafError> {
        AnnotationBuilder::new()
            .annotation_id(format!("a{}", annotation_index))
            .time_start((self.start * 1000.).round() as i64)
            .time_end((self.end * 1000.).round() as i64)
            .time_slot_ref1(format!("ts{}", timeslot_index))
            .time_slot_ref2(format!("ts{}", timeslot_index + 1))
            .annotation_value(self.text.trim())
            .build()
    }

    pub fn words_to_alignable(
        &self,
        start_index_annotation: usize,
        start_index_timeslot: usize
    ) -> Result<(Vec<Annotation>, Vec<TimeSlot>), EafError> {
        let mut annots: Vec<Annotation> = Vec::new();
        let mut ts: Vec<TimeSlot> = Vec::new();
        for word in self.words.iter() {
            annots.push(
                word.to_alignable(
                    start_index_annotation + annots.len(),
                    start_index_timeslot + ts.len()
                )?
            );
            ts.extend(word.to_timeslots(start_index_timeslot + ts.len()));
        }

        Ok((annots, ts))
    }

    /// Returns start, end time as ELAN time slots.
    pub fn to_timeslots(&self, start_index: usize) -> [TimeSlot; 2] {
        [
            TimeSlot::new(&format!("ts{}", start_index), Some((self.start * 1000.).round() as i64)),
            TimeSlot::new(&format!("ts{}", start_index + 1), Some((self.end * 1000.).round() as i64))
        ]
    }

    // /// Returns a referred ELAN annotation with confidence as annotation value.
    // pub fn avg_logprob_to_referred(
    //     &self,
    //     annotation_index: usize,
    //     annotation_ref_index: usize
    // ) -> Result<eaf_rs::Annotation, eaf_rs::EafError> {
    //     AnnotationBuilder::new()
    //         .annotation_id(format!("a{}", annotation_index))
    //         .time_start((self.start * 1000.).round() as i64)
    //         .time_end((self.end * 1000.).round() as i64)
    //         .annotation_ref(format!("a{}", annotation_ref_index))
    //         .annotation_value(&self.avg_logprob.to_string())
    //         .build()
    // }

    pub fn to_referred(
        &self,
        annotation_index: usize,
        annotation_ref_index: usize,
        field_type: WhisperFieldType
    ) -> Result<eaf_rs::Annotation, eaf_rs::EafError> {
        AnnotationBuilder::new()
            .annotation_id(format!("a{}", annotation_index))
            .time_start((self.start * 1000.).round() as i64)
            .time_end((self.end * 1000.).round() as i64)
            .annotation_ref(format!("a{}", annotation_ref_index))
            .annotation_value(field_type.to_string(&self).trim())
            .build()
    }

    /// Adjust timestamps by specified amount of seconds.
    pub fn offset(self, offset: f64) -> Self {
        Self {
            start: self.start + offset,
            end: self.end + offset,
            words: self.words
                .into_iter()
                .map(|w| w.offset(offset)).collect(),
            ..self
        }
    }
}

/// Whisper single word (only available if `word_timestamps == True`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WhisperWord {
    /// End time in seconds.
    pub end: f64, // 0.56,
    /// Probability.
    pub probability: f64, // 0.
    /// Start time in seconds.
    pub start: f64, // 0.36,
    /// Value.
    pub word: String, // "no,",
}

impl WhisperWord {
    /// Returns an aligned ELAN annotation.
    pub fn to_alignable(
        &self,
        annotation_index: usize,
        timeslot_index: usize
    ) -> Result<eaf_rs::Annotation, eaf_rs::EafError> {
        AnnotationBuilder::new()
            .annotation_id(format!("a{}", annotation_index))
            .time_start((self.start * 1000.).round() as i64)
            .time_end((self.end * 1000.).round() as i64)
            .time_slot_ref1(format!("ts{}", timeslot_index))
            .time_slot_ref2(format!("ts{}", timeslot_index + 1))
            .annotation_value(self.word.trim())
            .build()
    }

    /// Returns start, end time as ELAN time slots.
    pub fn to_timeslots(&self, timeslot_index: usize) -> [TimeSlot; 2] {
        [
            TimeSlot::new(&format!("ts{}", timeslot_index), Some((self.start * 1000.).round() as i64)),
            TimeSlot::new(&format!("ts{}", timeslot_index + 1), Some((self.end * 1000.).round() as i64))
        ]
    }

    /// Adjust timestamps by specified amount of seconds.
    pub fn offset(self, offset: f64) -> Self {
        Self {
            start: self.start + offset,
            end: self.end + offset,
            ..self
        }
    }
}
