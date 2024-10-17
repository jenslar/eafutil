//! Data structures corresponding to Whisper timestamped JSON-files.
//! The major difference to normal Whisper output is word level timing,
//! with sub-second time stamps.
//! See: <https://github.com/linto-ai/whisper-timestamped>

use std::{fs::{read_to_string, File}, io::Write, path::{Path, PathBuf}};

use eaf_rs::{Eaf, Tier, eaf::annotation::AnnotationBuilder, Annotation, TimeSlot, EafError, TimeOrder, StereoType, LinguisticType};
use serde::{self, Deserialize, Serialize};
use serde_json;

use crate::clips::Clips;

/// Whisper Timestamped JSON structure.
/// 
/// See: <https://github.com/linto-ai/whisper-timestamped>
/// 
/// Whisper with word/token level sub-second timestamps.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WhisperTsJson {
    /// Full text
    pub text: String,
    /// Segmented text.
    pub segments: Vec<WhisperTsSegment>
}

impl WhisperTsJson {
    /// Read whisper timestamped JSON-file (word level timing).
    /// See: <https://github.com/linto-ai/whisper-timestamped>
    /// 
    /// ```json
    /// {
    ///  "text": "No, no, no, wait, wait, wait, wait, there are some other cars ...",
    ///  "segments": [
    ///    {
    ///      "start": 0.0,
    ///      "end": 3.64,
    ///      "text": "No, no, no, wait, wait, wait, wait, there are some other cars",
    ///      "confidence": 0.65,
    ///      "words": [
    ///        {
    ///          "text": "No,",
    ///          "start": 0.0,
    ///          "end": 0.36,
    ///          "confidence": 0.3
    ///        },
    ///        ...
    ///    }
    ///    {
    ///      "start": 4.9,
    ///      "end": 5.88,
    ///      "text": ...,
    ///      ...
    ///    },
    ///    ...
    ///   ]
    ///   "language": "en",
    ///   "language_probs": {
    ///     "en": 0.6008778810501099,
    ///     "zh": 0.0008387790294364095,
    ///     ...
    ///   }
    /// }
    ///    
    /// ```
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
        let mut text: Vec<&str> = vec![self.text.as_str()];
        let mut segments: Vec<WhisperTsSegment> = self.segments.to_vec();
        for w in others.iter_mut() {
            text.push(w.text.as_str());
            // let ts = w.last_end()
            segments.append(&mut w.segments);
        }

        Ok(Self {
            text: text.join(""),
            segments: segments,
            ..self.to_owned()
        })
    }

    /// Reads multiple whisper timestamped JSON-files
    /// and combines these into a single `WhisperTsJson`.
    /// 
    /// Note that detected language and it probabilities
    /// will be set to those of the first file
    /// if the input contatins multiple languages.
    /// I.e 
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
    // pub fn from_paths(paths: &[PathBuf], clips: &Clips) -> std::io::Result<Self> {
    //     assert_eq!(paths.len(), clips.len());
    //     let first = paths.first()
    //         .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "No JSON-paths provived"))?;
    //     let head = Self::read(&first)?;
    //     let mut tail = paths.iter().enumerate().skip(1)
    //         .map(|p| {
    //             clips.get_timestamps(&)
    //             Self::read(p)
    //         })
    //         .collect::<std::io::Result<Vec<Self>>>()?;

    //     head.join(&mut tail)
    // }

    /// Returns the first segment's start timestamp.
    /// Note that this does not correspond to start
    /// of the corresponding media file.
    pub fn first_start(&self) -> Option<f64> {
        Some(self.segments.first()?.start)
    }

    /// Returns the last segment's start timestamp.
    /// Note that this does not correspond to start
    /// of the corresponding media file.
    pub fn last_start(&self) -> Option<f64> {
        Some(self.segments.last()?.start)
    }

    /// Returns the first segment's end timestamp.
    /// Note that this does not correspond to end
    /// of the corresponding media file.
    pub fn first_end(&self) -> Option<f64> {
        Some(self.segments.first()?.end)
    }

    /// Returns the last segment's end timestamp.
    /// Note that this does not correspond to end
    /// of the corresponding media file.
    pub fn last_end(&self) -> Option<f64> {
        Some(self.segments.last()?.end)
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        true
    }

    /// Generates EAF with three tiers:
    /// - `segments`: full speech segment
    /// - `words`: aligned word tokens (within timespan of corresponding segment)
    ///  `confidence`: confidence of speech segment
    pub fn to_eaf(&self) -> Result<Eaf, EafError> {
        let mut eaf = Eaf::default();

        let segments_id = "segments";
        let words_id = "words";
        let confidence_id = "confidence";

        let mut main_annotations: Vec<Annotation> = Vec::new();
        let mut ref_annotations_words: Vec<Annotation> = Vec::new();
        let mut ref_annotations_conf: Vec<Annotation> = Vec::new();
        let mut timeslots: Vec<TimeSlot> = Vec::new();

        for segment in self.segments.iter() {
            let base_annot_idx = 1
            + main_annotations.len()
            + ref_annotations_words.len()
            + ref_annotations_conf.len();
            let base_ts_idx = 1 + timeslots.len();

            // Generate timeslots
            // Generate main annotation
            main_annotations.push(segment.to_alignable(base_annot_idx, base_ts_idx)?);
            // Add main annotation timeslots
            timeslots.extend(segment.to_timeslots(base_ts_idx));

            
            // Generate ref annotations with words, alignable annotations (within parent tier annotation timespans)
            let (words, words_ts) = segment.words_to_alignable(base_annot_idx + 1, base_ts_idx + 2)?;
            let a_id_conf_index = base_annot_idx + words.len() + 1;
            
            ref_annotations_words.extend(words);

            timeslots.extend(words_ts);
            
            // Generate ref_annotations for confidence
            ref_annotations_conf.push(segment.confidence_to_referred(a_id_conf_index, base_annot_idx)?)
        }

        let lingtype_words = LinguisticType::new(words_id, Some(&StereoType::IncludedIn));
        let lingtype_conf = LinguisticType::new(confidence_id, Some(&StereoType::SymbolicAssociation));
        
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
        let ref_tier_conf = Tier::new(
            confidence_id,
            Some(&ref_annotations_conf),
            Some(confidence_id),
            Some(segments_id)
        );

        eaf.time_order = TimeOrder { time_slots: timeslots };
        eaf.tiers.push(main_tier);
        eaf.tiers.push(ref_tier_words);
        eaf.tiers.push(ref_tier_conf);
        eaf.add_linguistic_type(&lingtype_words, true);
        eaf.add_linguistic_type(&lingtype_conf, true);

        Ok(eaf)
    }
}

/// Whisper Timestamped segment.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WhisperTsSegment {
    /// Start time in seconds.
    pub start: f64,// 0.0,
    /// End time in seconds.
    pub end: f64,// 3.64,
    /// Full speech segment.
    pub text: String,
    /// Confidence.
    pub confidence: f64,// 0.65,
    /// Segmented words.
    pub words: Vec<WhisperTsWord>
}

impl WhisperTsSegment {
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
            .annotation_value(&self.text)
            .build()
    }

    /// Returns a referred ELAN annotation.
    pub fn to_referred(
        &self,
        annotation_index: usize,
        annotation_ref_index: usize
    ) -> Result<Annotation, EafError> {
        AnnotationBuilder::new()
            .annotation_id(format!("a{}", annotation_index))
            .time_start((self.start * 1000.).round() as i64)
            .time_end((self.end * 1000.).round() as i64)
            .annotation_ref(format!("a{}", annotation_ref_index))
            .annotation_value(&self.text)
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

    /// Returns a referred ELAN annotation with confidence as annotation value.
    pub fn confidence_to_referred(&self,
        annotation_index: usize,
        annotation_ref_index: usize
    ) -> Result<eaf_rs::Annotation, eaf_rs::EafError> {
        AnnotationBuilder::new()
            .annotation_id(format!("a{}", annotation_index))
            .time_start((self.start * 1000.).round() as i64)
            .time_end((self.end * 1000.).round() as i64)
            .annotation_ref(format!("a{}", annotation_ref_index))
            .annotation_value(&self.confidence.to_string())
            .build()
    }

    /// Returns start, end time as ELAN time slots.
    pub fn to_timeslots(&self, start_index: usize) -> [TimeSlot; 2] {
        [
            TimeSlot::new(&format!("ts{}", start_index), Some((self.start * 1000.).round() as i64)),
            TimeSlot::new(&format!("ts{}", start_index + 1), Some((self.end * 1000.).round() as i64))
        ]
    }

    /// Returns timespan of segment in seconds.
    pub fn time_span(&self) -> f64 {
        self.end - self.start
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

/// Whisper Timestamped single word.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WhisperTsWord {
    /// Value.
    pub text: String, // "no,",
    /// Start time in seconds.
    pub start: f64, // 0.36,
    /// End time in seconds.
    pub end: f64, // 0.56,
    /// Confidence.
    pub confidence: f64, // 0.
}

impl WhisperTsWord {
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
            .annotation_value(&self.text)
            .build()
    }

    /// Returns a referred ELAN annotation.
    pub fn to_referred(
        &self,
        annotation_index: usize,
        annotation_ref_index: usize
    ) -> Result<eaf_rs::Annotation, eaf_rs::EafError> {
        AnnotationBuilder::new()
            .annotation_id(format!("a{}", annotation_index))
            .time_start((self.start * 1000.).round() as i64)
            .time_end((self.end * 1000.).round() as i64)
            .annotation_ref(format!("a{}", annotation_ref_index))
            .annotation_value(&self.text)
            .build()
    }

    /// Returns start, end time as ELAN time slots.
    pub fn to_timeslots(&self, timeslot_index: usize) -> [TimeSlot; 2] {
        [
            TimeSlot::new(&format!("ts{}", timeslot_index), Some((self.start * 1000.).round() as i64)),
            TimeSlot::new(&format!("ts{}", timeslot_index + 1), Some((self.end * 1000.).round() as i64))
        ]
    }

    /// Returns timespan of word in seconds.
    pub fn time_span(&self) -> f64 {
        self.end - self.start
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
