use exif::{Reader, Tag, Value};
use id3::TagLike;
use ignore::WalkBuilder;
use pdf::file::FileOptions;
use rayon::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::{fs, path::PathBuf, time::SystemTime};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Default)]
pub struct FileMetadata {
    // General file information
    pub file_name: String,
    pub file_path: PathBuf,
    pub file_extension: String,
    pub file_size: u64,
    pub mime_type: String,

    // Timestamps
    pub creation_time: Option<SystemTime>,
    pub modification_time: Option<SystemTime>,
    pub access_time: Option<SystemTime>,

    // Common document metadata
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub author: Option<String>,
    pub title: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub pages: Option<u32>,

    // Image specific
    pub location: Option<(f64, f64)>,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub resolution: Option<(u32, u32)>,
    pub orientation: Option<u16>,

    // Audio/Video specific
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_title: Option<String>,
    pub genre: Option<String>,
    pub year: Option<i32>,
    pub track_number: Option<u32>,
    pub duration: Option<u32>,    // in seconds
    pub bitrate: Option<u32>,     // in kbps
    pub sample_rate: Option<u32>, // in Hz
    pub frame_rate: Option<f32>,  // for video, frames per second
}

impl FileMetadata {
    pub fn new(file_path: &PathBuf) -> Self {
        let metadata = fs::metadata(file_path).unwrap();
        let mime_type = mime_guess::from_path(file_path).first_or_octet_stream();

        let mut file_metadata = FileMetadata {
            file_name: file_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            file_path: file_path.canonicalize().unwrap_or(file_path.clone()),
            file_size: metadata.len(),
            file_extension: file_path
                .extension()
                .unwrap_or(OsStr::new(""))
                .to_str()
                .unwrap()
                .to_string(),
            mime_type: mime_type.to_string(),
            creation_time: metadata.created().ok(),
            modification_time: metadata.modified().ok(),
            access_time: metadata.accessed().ok(),
            ..Default::default()
        };

        if file_metadata.file_extension.to_lowercase() == "pdf" {
            file_metadata = file_metadata.from_pdf();
        }

        if mime_type.type_() == mime_guess::mime::IMAGE {
            file_metadata = file_metadata.from_img();
        }

        if mime_type.type_() == mime_guess::mime::AUDIO {
            file_metadata = file_metadata.from_audio();
        }

        if mime_type.type_() == mime_guess::mime::VIDEO {
            file_metadata = file_metadata.from_mp4();
        }

        file_metadata
    }

    pub fn from_folder(folder_path: &PathBuf) -> Vec<FileMetadata> {
        let metadata = WalkBuilder::new(folder_path)
            .git_ignore(true)
            .hidden(true)
            .ignore(true)
            .build()
            .par_bridge()
            .filter_map(|result| {
                result.ok().and_then(|entry| {
                    let path = entry.path().to_path_buf();
                    if path.is_file() {
                        Some(FileMetadata::new(&path))
                    } else {
                        None
                    }
                })
            })
            .collect::<Vec<FileMetadata>>();

        metadata
    }

    fn from_pdf(&mut self) -> Self {
        let doc = FileOptions::cached().open(&self.file_path).ok();
        if let Some(doc) = doc {
            if let Some(ref info) = doc.trailer.info_dict {
                if let Some(author) = &info.author {
                    self.author = author.to_string().ok();
                };

                if let Some(creator) = &info.creator {
                    self.creator = creator.to_string().ok();
                };

                if let Some(producer) = &info.producer {
                    self.producer = producer.to_string().ok();
                };

                if let Some(title) = &info.title {
                    self.title = title.to_string().ok();
                };

                if let Some(subject) = &info.subject {
                    self.subject = subject.to_string().ok();
                };

                if let Some(keywords) = &info.keywords {
                    self.keywords = keywords.to_string().ok();
                };
            }

            self.pages = Some(doc.pages().count() as u32);
        }

        self.clone()
    }

    fn from_img(&mut self) -> Self {
        if let Ok(file) = fs::File::open(&self.file_path) {
            let mut buf_reader = std::io::BufReader::new(file);
            let exif_reader = Reader::new().read_from_container(&mut buf_reader).ok();
            if exif_reader.is_none() {
                return self.clone();
            }

            let exif_reader = exif_reader.unwrap();
            for field in exif_reader.fields() {
                match field.tag {
                    Tag::Artist => {
                        self.author = field
                            .display_value()
                            .with_unit(&exif_reader)
                            .to_string()
                            .into()
                    }
                    Tag::ImageWidth => {
                        if let Some(width) = field.value.get_uint(0) {
                            self.resolution =
                                Some((width, self.resolution.map(|(_, h)| h).unwrap_or(0)));
                        }
                    }
                    Tag::ImageLength => {
                        if let Some(height) = field.value.get_uint(0) {
                            self.resolution =
                                Some((self.resolution.map(|(w, _)| w).unwrap_or(0), height));
                        }
                    }
                    Tag::GPSLatitude => {
                        println!("Latitude: {:?}", field.value);
                        if let Value::Rational(rational) = &field.value {
                            if let Some(lat) = rational.first() {
                                let latitude = lat.to_f64();
                                self.location = Some((
                                    latitude,
                                    self.location.map(|(_, lon)| lon).unwrap_or(0.0),
                                ));
                            }
                        }
                    }
                    Tag::GPSLongitude => {
                        println!("Longitude: {:?}", field.value);
                        if let Value::Rational(rational) = &field.value {
                            if let Some(lon) = rational.first() {
                                let longitude = lon.to_f64();
                                self.location = Some((
                                    self.location.map(|(lat, _)| lat).unwrap_or(0.0),
                                    longitude,
                                ));
                            }
                        }
                    }
                    Tag::Model => {
                        if let Value::Ascii(model) = &field.value {
                            if let Some(model_str) = model
                                .first()
                                .and_then(|s| String::from_utf8(s.clone()).ok())
                            {
                                self.camera_model = Some(model_str);
                            }
                        }
                    }
                    Tag::Make => {
                        if let Value::Ascii(make) = &field.value {
                            if let Some(make_str) =
                                make.first().and_then(|s| String::from_utf8(s.clone()).ok())
                            {
                                self.camera_make = Some(make_str);
                            }
                        }
                    }
                    Tag::Orientation => {
                        if let Value::Short(orientation) = &field.value {
                            if let Some(&orientation_value) = orientation.first() {
                                self.orientation = Some(orientation_value);
                            }
                        }
                    }
                    _ => {}
                };
            }
        }

        self.clone()
    }

    fn from_audio(&mut self) -> Self {
        if let Ok(tag) = id3::Tag::read_from_path(&self.file_path) {
            self.artist = tag.artist().map(|s| s.to_string());
            self.album = tag.album().map(|s| s.to_string());
            self.track_title = tag.title().map(|s| s.to_string());
            self.genre = tag.genre().map(|s| s.to_string());
            self.year = tag.year();
            self.track_number = tag.track();
            self.duration = tag.duration();
        }

        self.clone()
    }

    fn from_mp4(&mut self) -> Self {
        if let Ok(tag) = mp4ameta::Tag::read_from_path(&self.file_path) {
            self.artist = tag.artist().map(|s| s.to_string());
            self.album = tag.album().map(|s| s.to_string());
            self.track_title = tag.title().map(|s| s.to_string());
            self.genre = tag.genre().map(|s| s.to_string());
            self.keywords = Some(
                tag.keywords()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
            );
        }
        self.clone()
    }
}
