// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Model Modality Support
//!
//! Defines modalities for different types of AI models:
//! - LLM (Language Models) - Text-only
//! - VLM (Vision-Language Models) - Text + Images
//! - VLA (Vision-Language-Action Models) - Text + Images + Actions/Robotics
//! - ALM (Audio-Language Models) - Text + Audio
//! - VALM (Video-Audio-Language Models) - Text + Video + Audio

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::models::MessageRole;

// =============================================================================
// Core Modality Types
// =============================================================================

/// Input/Output modality types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    /// Text input/output
    Text,
    /// Image input/output
    Image,
    /// Video input/output
    Video,
    /// Audio input/output
    Audio,
    /// 3D point cloud or mesh
    PointCloud,
    /// Action commands (for robotics/VLA)
    Action,
    /// Sensor data (proprioception, IMU, etc.)
    Sensor,
    /// Depth map
    Depth,
    /// Semantic segmentation
    Segmentation,
    /// Bounding boxes
    BoundingBox,
    /// Pose estimation
    Pose,
    /// Trajectory/path data
    Trajectory,
}

impl std::fmt::Display for Modality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Modality::Text => write!(f, "text"),
            Modality::Image => write!(f, "image"),
            Modality::Video => write!(f, "video"),
            Modality::Audio => write!(f, "audio"),
            Modality::PointCloud => write!(f, "point_cloud"),
            Modality::Action => write!(f, "action"),
            Modality::Sensor => write!(f, "sensor"),
            Modality::Depth => write!(f, "depth"),
            Modality::Segmentation => write!(f, "segmentation"),
            Modality::BoundingBox => write!(f, "bounding_box"),
            Modality::Pose => write!(f, "pose"),
            Modality::Trajectory => write!(f, "trajectory"),
        }
    }
}

/// Model category based on modality support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ModelCategory {
    /// Language Model - Text only
    LLM,
    /// Vision-Language Model - Text + Images
    VLM,
    /// Vision-Language-Action Model - Text + Images + Actions
    VLA,
    /// Audio-Language Model - Text + Audio  
    ALM,
    /// Video-Audio-Language Model - Text + Video + Audio
    VALM,
    /// Multimodal - supports multiple modalities
    Multimodal,
    /// Embodied AI - for robotics with full sensor/action support
    Embodied,
}

impl std::fmt::Display for ModelCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelCategory::LLM => write!(f, "LLM"),
            ModelCategory::VLM => write!(f, "VLM"),
            ModelCategory::VLA => write!(f, "VLA"),
            ModelCategory::ALM => write!(f, "ALM"),
            ModelCategory::VALM => write!(f, "VALM"),
            ModelCategory::Multimodal => write!(f, "Multimodal"),
            ModelCategory::Embodied => write!(f, "Embodied"),
        }
    }
}

// =============================================================================
// Modality Capabilities
// =============================================================================

/// Describes what modalities a model can accept and produce
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalityCapabilities {
    /// Model category
    pub category: ModelCategory,
    /// Supported input modalities
    pub input_modalities: Vec<Modality>,
    /// Supported output modalities
    pub output_modalities: Vec<Modality>,
    /// Maximum image resolution (width, height)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_image_resolution: Option<(u32, u32)>,
    /// Maximum video duration in seconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_video_duration: Option<u32>,
    /// Maximum audio duration in seconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_audio_duration: Option<u32>,
    /// Maximum images per request
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_images_per_request: Option<u32>,
    /// Supported image formats
    #[serde(default)]
    pub supported_image_formats: Vec<ImageFormat>,
    /// Supports real-time streaming
    #[serde(default)]
    pub supports_streaming: bool,
    /// Supports interleaved multi-turn with images
    #[serde(default)]
    pub supports_interleaved: bool,
}

impl Default for ModalityCapabilities {
    fn default() -> Self {
        Self {
            category: ModelCategory::LLM,
            input_modalities: vec![Modality::Text],
            output_modalities: vec![Modality::Text],
            max_image_resolution: None,
            max_video_duration: None,
            max_audio_duration: None,
            max_images_per_request: None,
            supported_image_formats: vec![],
            supports_streaming: false,
            supports_interleaved: false,
        }
    }
}

impl ModalityCapabilities {
    /// Create capabilities for a text-only LLM
    pub fn llm() -> Self {
        Self {
            category: ModelCategory::LLM,
            input_modalities: vec![Modality::Text],
            output_modalities: vec![Modality::Text],
            supports_streaming: true,
            ..Default::default()
        }
    }

    /// Create capabilities for a Vision-Language Model
    pub fn vlm() -> Self {
        Self {
            category: ModelCategory::VLM,
            input_modalities: vec![Modality::Text, Modality::Image],
            output_modalities: vec![Modality::Text],
            max_image_resolution: Some((4096, 4096)),
            max_images_per_request: Some(20),
            supported_image_formats: vec![
                ImageFormat::Png,
                ImageFormat::Jpeg,
                ImageFormat::Webp,
                ImageFormat::Gif,
            ],
            supports_streaming: true,
            supports_interleaved: true,
            ..Default::default()
        }
    }

    /// Create capabilities for a Vision-Language-Action Model
    pub fn vla() -> Self {
        Self {
            category: ModelCategory::VLA,
            input_modalities: vec![
                Modality::Text,
                Modality::Image,
                Modality::Sensor,
                Modality::Depth,
            ],
            output_modalities: vec![Modality::Text, Modality::Action, Modality::Trajectory],
            max_image_resolution: Some((1024, 1024)),
            max_images_per_request: Some(10),
            supported_image_formats: vec![ImageFormat::Png, ImageFormat::Jpeg],
            supports_streaming: true,
            supports_interleaved: true,
            ..Default::default()
        }
    }

    /// Create capabilities for a multimodal model (like GPT-4o or Gemini)
    pub fn multimodal() -> Self {
        Self {
            category: ModelCategory::Multimodal,
            input_modalities: vec![
                Modality::Text,
                Modality::Image,
                Modality::Audio,
                Modality::Video,
            ],
            output_modalities: vec![Modality::Text, Modality::Image, Modality::Audio],
            max_image_resolution: Some((4096, 4096)),
            max_video_duration: Some(3600),
            max_audio_duration: Some(3600),
            max_images_per_request: Some(50),
            supported_image_formats: vec![
                ImageFormat::Png,
                ImageFormat::Jpeg,
                ImageFormat::Webp,
                ImageFormat::Gif,
            ],
            supports_streaming: true,
            supports_interleaved: true,
        }
    }

    /// Create capabilities for an embodied AI model
    pub fn embodied() -> Self {
        Self {
            category: ModelCategory::Embodied,
            input_modalities: vec![
                Modality::Text,
                Modality::Image,
                Modality::Depth,
                Modality::PointCloud,
                Modality::Sensor,
                Modality::Pose,
            ],
            output_modalities: vec![
                Modality::Text,
                Modality::Action,
                Modality::Trajectory,
                Modality::Pose,
            ],
            max_image_resolution: Some((1280, 720)),
            max_images_per_request: Some(8),
            supported_image_formats: vec![ImageFormat::Png, ImageFormat::Jpeg],
            supports_streaming: true,
            supports_interleaved: true,
            ..Default::default()
        }
    }

    /// Check if this model supports a given input modality
    pub fn supports_input(&self, modality: Modality) -> bool {
        self.input_modalities.contains(&modality)
    }

    /// Check if this model supports a given output modality
    pub fn supports_output(&self, modality: Modality) -> bool {
        self.output_modalities.contains(&modality)
    }

    /// Check if this model supports vision input
    pub fn supports_vision(&self) -> bool {
        self.supports_input(Modality::Image) || self.supports_input(Modality::Video)
    }

    /// Check if this model supports action output
    pub fn supports_actions(&self) -> bool {
        self.supports_output(Modality::Action) || self.supports_output(Modality::Trajectory)
    }
}

/// Supported image formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
    Gif,
    Bmp,
    Tiff,
    Heic,
}

impl std::fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageFormat::Png => write!(f, "png"),
            ImageFormat::Jpeg => write!(f, "jpeg"),
            ImageFormat::Webp => write!(f, "webp"),
            ImageFormat::Gif => write!(f, "gif"),
            ImageFormat::Bmp => write!(f, "bmp"),
            ImageFormat::Tiff => write!(f, "tiff"),
            ImageFormat::Heic => write!(f, "heic"),
        }
    }
}

// =============================================================================
// Multimodal Content Types
// =============================================================================

/// Image content for vision models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    /// Image data (base64 encoded or URL)
    pub data: ImageData,
    /// Image format
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<ImageFormat>,
    /// Image description/alt text
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt_text: Option<String>,
    /// Bounding box regions of interest
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<BoundingBoxRegion>,
}

/// Image data - either base64 encoded or URL reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ImageData {
    /// Base64 encoded image data
    Base64 {
        #[serde(rename = "base64")]
        data: String,
        media_type: String,
    },
    /// URL reference to image
    Url {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<ImageDetail>,
    },
}

/// Image detail level for vision models
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageDetail {
    /// Low resolution processing
    Low,
    /// High resolution processing
    High,
    /// Auto-select based on image size
    Auto,
}

/// Bounding box region in an image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBoxRegion {
    /// Region label
    pub label: String,
    /// Normalized coordinates (0.0 - 1.0)
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Confidence score (0.0 - 1.0)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
}

/// Video content for video models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoContent {
    /// Video URL or base64 data
    pub data: VideoData,
    /// Video duration in seconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f32>,
    /// Start time for clip (seconds)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<f32>,
    /// End time for clip (seconds)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<f32>,
    /// Frame rate
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fps: Option<f32>,
}

/// Video data - URL or uploaded frames
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VideoData {
    /// URL reference to video
    Url { url: String },
    /// Sequence of frames as images
    Frames { frames: Vec<ImageContent> },
    /// Base64 encoded video
    Base64 { base64: String, media_type: String },
}

/// Audio content for audio models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioContent {
    /// Audio data
    pub data: AudioData,
    /// Audio format (mp3, wav, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Duration in seconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f32>,
    /// Sample rate
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
    /// Transcription (if available)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcription: Option<String>,
}

/// Audio data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AudioData {
    /// URL reference to audio
    Url { url: String },
    /// Base64 encoded audio
    Base64 { base64: String, media_type: String },
}

// =============================================================================
// VLA (Vision-Language-Action) Types
// =============================================================================

/// Action command for VLA models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionCommand {
    /// Action type
    pub action_type: ActionType,
    /// Action parameters
    pub parameters: ActionParameters,
    /// Confidence score (0.0 - 1.0)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    /// Timestamp for this action
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
    /// Duration of action in milliseconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Types of robot actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    // Navigation actions
    Move,
    Rotate,
    Stop,

    // Manipulation actions
    Grasp,
    Release,
    Push,
    Pull,
    Place,
    Pick,

    // End effector actions
    Open,
    Close,

    // Arm actions
    MoveArm,
    MoveJoint,

    // Camera actions
    Look,
    Focus,

    // Generic
    Custom,
    Wait,
    Sequence,
}

/// Parameters for different action types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ActionParameters {
    /// Movement parameters (x, y, z displacement or velocity)
    Movement {
        #[serde(default)]
        x: f64,
        #[serde(default)]
        y: f64,
        #[serde(default)]
        z: f64,
        /// Whether values are velocities or positions
        #[serde(default)]
        is_velocity: bool,
        /// Coordinate frame (world, robot, end_effector)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        frame: Option<String>,
    },
    /// Rotation parameters (roll, pitch, yaw or quaternion)
    Rotation {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        roll: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pitch: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        yaw: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        quaternion: Option<[f64; 4]>,
    },
    /// Gripper parameters
    Gripper {
        /// Aperture (0.0 = closed, 1.0 = fully open)
        aperture: f64,
        /// Force limit
        #[serde(default, skip_serializing_if = "Option::is_none")]
        force: Option<f64>,
    },
    /// Joint positions
    JointPositions {
        /// Joint angles in radians
        positions: Vec<f64>,
        /// Joint names (if applicable)
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        joint_names: Vec<String>,
    },
    /// Target pose (position + orientation)
    TargetPose {
        position: [f64; 3],
        /// Quaternion [w, x, y, z]
        orientation: [f64; 4],
    },
    /// Trajectory of waypoints
    Trajectory {
        waypoints: Vec<Waypoint>,
        /// Interpolation method
        #[serde(default, skip_serializing_if = "Option::is_none")]
        interpolation: Option<String>,
    },
    /// Custom parameters as JSON
    Custom(serde_json::Value),
}

/// Waypoint in a trajectory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Waypoint {
    /// Position [x, y, z]
    pub position: [f64; 3],
    /// Orientation quaternion [w, x, y, z]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<[f64; 4]>,
    /// Timestamp offset in seconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<f64>,
    /// Gripper state at this waypoint
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gripper: Option<f64>,
}

/// Sensor data input for VLA models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorData {
    /// Sensor type
    pub sensor_type: SensorType,
    /// Sensor values
    pub values: SensorValues,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Sensor frame/reference
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
}

/// Types of sensors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorType {
    /// Joint positions/velocities
    JointState,
    /// Inertial measurement unit
    Imu,
    /// Force/torque sensor
    ForceTorque,
    /// Depth camera
    Depth,
    /// LIDAR
    Lidar,
    /// GPS/Localization
    Localization,
    /// Touch/pressure sensor
    Tactile,
    /// Odometry
    Odometry,
    /// Custom sensor
    Custom,
}

/// Sensor values for different sensor types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SensorValues {
    /// Joint state (positions, velocities, efforts)
    JointState {
        positions: Vec<f64>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        velocities: Vec<f64>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        efforts: Vec<f64>,
    },
    /// IMU (acceleration, angular velocity)
    Imu {
        linear_acceleration: [f64; 3],
        angular_velocity: [f64; 3],
        #[serde(default, skip_serializing_if = "Option::is_none")]
        orientation: Option<[f64; 4]>,
    },
    /// Force/torque (6D wrench)
    ForceTorque { force: [f64; 3], torque: [f64; 3] },
    /// Depth map (as base64 or URL)
    Depth {
        data: String,
        width: u32,
        height: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        encoding: Option<String>,
    },
    /// Point cloud
    PointCloud {
        points: Vec<[f64; 3]>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        colors: Vec<[u8; 3]>,
    },
    /// Pose (position + orientation)
    Pose {
        position: [f64; 3],
        orientation: [f64; 4],
    },
    /// Generic numeric values
    Numeric(Vec<f64>),
    /// Custom values as JSON
    Custom(serde_json::Value),
}

// =============================================================================
// Multimodal Message Content
// =============================================================================

/// Content part of a multimodal message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// Text content
    Text { text: String },
    /// Image content
    Image(ImageContent),
    /// Video content
    Video(VideoContent),
    /// Audio content
    Audio(AudioContent),
    /// Sensor data (for VLA)
    Sensor(SensorData),
    /// Action command (for VLA output)
    Action(ActionCommand),
    /// File reference
    File {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
}

/// A multimodal message that can contain mixed content types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimodalMessage {
    /// Message role
    pub role: MessageRole,
    /// Content parts
    pub content: Vec<ContentPart>,
    /// Timestamp
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl MultimodalMessage {
    /// Create a new text-only message
    pub fn text(role: MessageRole, text: impl Into<String>) -> Self {
        Self {
            role,
            content: vec![ContentPart::Text { text: text.into() }],
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create a message with text and image
    pub fn with_image(role: MessageRole, text: impl Into<String>, image: ImageContent) -> Self {
        Self {
            role,
            content: vec![
                ContentPart::Text { text: text.into() },
                ContentPart::Image(image),
            ],
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Add an image to this message
    pub fn add_image(&mut self, image: ImageContent) {
        self.content.push(ContentPart::Image(image));
    }

    /// Add sensor data to this message
    pub fn add_sensor(&mut self, sensor: SensorData) {
        self.content.push(ContentPart::Sensor(sensor));
    }

    /// Add an action to this message
    pub fn add_action(&mut self, action: ActionCommand) {
        self.content.push(ContentPart::Action(action));
    }

    /// Get all text content concatenated
    pub fn text_content(&self) -> String {
        self.content
            .iter()
            .filter_map(|part| match part {
                ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get all images in this message
    pub fn images(&self) -> Vec<&ImageContent> {
        self.content
            .iter()
            .filter_map(|part| match part {
                ContentPart::Image(img) => Some(img),
                _ => None,
            })
            .collect()
    }

    /// Get all actions in this message
    pub fn actions(&self) -> Vec<&ActionCommand> {
        self.content
            .iter()
            .filter_map(|part| match part {
                ContentPart::Action(action) => Some(action),
                _ => None,
            })
            .collect()
    }
}

// =============================================================================
// VLM/VLA Model Registry
// =============================================================================

/// Known VLM/VLA model with capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimodalModel {
    /// Model identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Provider name
    pub provider: String,
    /// Model category
    pub category: ModelCategory,
    /// Modality capabilities
    pub capabilities: ModalityCapabilities,
    /// Maximum context length (tokens)
    pub max_context: u32,
    /// Model version
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Release date
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    /// Model description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Pricing info
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pricing: Option<ModelPricing>,
    /// Whether model is available via API
    #[serde(default)]
    pub available: bool,
    /// Whether model can run locally
    #[serde(default)]
    pub local: bool,
}

/// Model pricing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Cost per million input tokens
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_per_million: Option<f64>,
    /// Cost per million output tokens
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_per_million: Option<f64>,
    /// Cost per image
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_image: Option<f64>,
    /// Cost per minute of video
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_video_minute: Option<f64>,
    /// Cost per minute of audio
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_audio_minute: Option<f64>,
    /// Currency
    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_currency() -> String {
    "USD".to_string()
}

/// Get built-in VLM models
pub fn vlm_models() -> Vec<MultimodalModel> {
    vec![
        // OpenAI VLMs
        MultimodalModel {
            id: "gpt-4o".to_string(),
            name: "GPT-4o".to_string(),
            provider: "OpenAI".to_string(),
            category: ModelCategory::Multimodal,
            capabilities: ModalityCapabilities::multimodal(),
            max_context: 128000,
            version: Some("2024-11-20".to_string()),
            release_date: Some("2024-05-13".to_string()),
            description: Some("Most capable GPT-4 with vision, audio, and text".to_string()),
            pricing: Some(ModelPricing {
                input_per_million: Some(2.50),
                output_per_million: Some(10.00),
                per_image: None,
                per_video_minute: None,
                per_audio_minute: None,
                currency: "USD".to_string(),
            }),
            available: true,
            local: false,
        },
        MultimodalModel {
            id: "gpt-4o-mini".to_string(),
            name: "GPT-4o Mini".to_string(),
            provider: "OpenAI".to_string(),
            category: ModelCategory::VLM,
            capabilities: ModalityCapabilities::vlm(),
            max_context: 128000,
            version: Some("2024-07-18".to_string()),
            release_date: Some("2024-07-18".to_string()),
            description: Some("Affordable small model with vision capabilities".to_string()),
            pricing: Some(ModelPricing {
                input_per_million: Some(0.15),
                output_per_million: Some(0.60),
                per_image: None,
                per_video_minute: None,
                per_audio_minute: None,
                currency: "USD".to_string(),
            }),
            available: true,
            local: false,
        },
        // Google VLMs
        MultimodalModel {
            id: "gemini-2.0-flash".to_string(),
            name: "Gemini 2.0 Flash".to_string(),
            provider: "Google".to_string(),
            category: ModelCategory::Multimodal,
            capabilities: ModalityCapabilities::multimodal(),
            max_context: 1000000,
            version: Some("2.0".to_string()),
            release_date: Some("2024-12-11".to_string()),
            description: Some("Fastest Gemini with native multimodal generation".to_string()),
            pricing: Some(ModelPricing {
                input_per_million: Some(0.075),
                output_per_million: Some(0.30),
                per_image: None,
                per_video_minute: None,
                per_audio_minute: None,
                currency: "USD".to_string(),
            }),
            available: true,
            local: false,
        },
        MultimodalModel {
            id: "gemini-1.5-pro".to_string(),
            name: "Gemini 1.5 Pro".to_string(),
            provider: "Google".to_string(),
            category: ModelCategory::Multimodal,
            capabilities: ModalityCapabilities::multimodal(),
            max_context: 2000000,
            version: Some("1.5".to_string()),
            release_date: Some("2024-02-15".to_string()),
            description: Some("2M context window with video understanding".to_string()),
            pricing: Some(ModelPricing {
                input_per_million: Some(1.25),
                output_per_million: Some(5.00),
                per_image: None,
                per_video_minute: None,
                per_audio_minute: None,
                currency: "USD".to_string(),
            }),
            available: true,
            local: false,
        },
        // Anthropic VLMs
        MultimodalModel {
            id: "claude-3-5-sonnet".to_string(),
            name: "Claude 3.5 Sonnet".to_string(),
            provider: "Anthropic".to_string(),
            category: ModelCategory::VLM,
            capabilities: ModalityCapabilities::vlm(),
            max_context: 200000,
            version: Some("20241022".to_string()),
            release_date: Some("2024-10-22".to_string()),
            description: Some("Best overall Claude with strong vision".to_string()),
            pricing: Some(ModelPricing {
                input_per_million: Some(3.00),
                output_per_million: Some(15.00),
                per_image: None,
                per_video_minute: None,
                per_audio_minute: None,
                currency: "USD".to_string(),
            }),
            available: true,
            local: false,
        },
        // Local/Open VLMs
        MultimodalModel {
            id: "llava-1.6".to_string(),
            name: "LLaVA 1.6".to_string(),
            provider: "Open Source".to_string(),
            category: ModelCategory::VLM,
            capabilities: ModalityCapabilities::vlm(),
            max_context: 4096,
            version: Some("1.6".to_string()),
            release_date: Some("2024-01-30".to_string()),
            description: Some("Open-source vision-language model".to_string()),
            pricing: None,
            available: true,
            local: true,
        },
        MultimodalModel {
            id: "qwen2-vl".to_string(),
            name: "Qwen2-VL".to_string(),
            provider: "Alibaba".to_string(),
            category: ModelCategory::VLM,
            capabilities: {
                let mut caps = ModalityCapabilities::vlm();
                caps.input_modalities.push(Modality::Video);
                caps
            },
            max_context: 32768,
            version: Some("2.0".to_string()),
            release_date: Some("2024-08-29".to_string()),
            description: Some("Strong open VLM with video understanding".to_string()),
            pricing: None,
            available: true,
            local: true,
        },
        MultimodalModel {
            id: "pixtral-12b".to_string(),
            name: "Pixtral 12B".to_string(),
            provider: "Mistral".to_string(),
            category: ModelCategory::VLM,
            capabilities: ModalityCapabilities::vlm(),
            max_context: 128000,
            version: Some("1.0".to_string()),
            release_date: Some("2024-09-11".to_string()),
            description: Some("Mistral's vision model, runs locally".to_string()),
            pricing: None,
            available: true,
            local: true,
        },
    ]
}

/// Get built-in VLA models
pub fn vla_models() -> Vec<MultimodalModel> {
    vec![
        MultimodalModel {
            id: "rt-2".to_string(),
            name: "RT-2".to_string(),
            provider: "Google DeepMind".to_string(),
            category: ModelCategory::VLA,
            capabilities: ModalityCapabilities::vla(),
            max_context: 4096,
            version: Some("2.0".to_string()),
            release_date: Some("2023-07-28".to_string()),
            description: Some("Robotics Transformer 2 - vision-language-action model".to_string()),
            pricing: None,
            available: false,
            local: false,
        },
        MultimodalModel {
            id: "rt-x".to_string(),
            name: "RT-X".to_string(),
            provider: "Open X-Embodiment".to_string(),
            category: ModelCategory::VLA,
            capabilities: ModalityCapabilities::vla(),
            max_context: 4096,
            version: Some("1.0".to_string()),
            release_date: Some("2023-10-05".to_string()),
            description: Some("Cross-embodiment robotics foundation model".to_string()),
            pricing: None,
            available: true,
            local: true,
        },
        MultimodalModel {
            id: "octo".to_string(),
            name: "Octo".to_string(),
            provider: "Berkeley AI Research".to_string(),
            category: ModelCategory::VLA,
            capabilities: ModalityCapabilities::vla(),
            max_context: 2048,
            version: Some("1.0".to_string()),
            release_date: Some("2024-05-10".to_string()),
            description: Some("Generalist robot policy from Open X-Embodiment".to_string()),
            pricing: None,
            available: true,
            local: true,
        },
        MultimodalModel {
            id: "openvla".to_string(),
            name: "OpenVLA".to_string(),
            provider: "Stanford/Berkeley".to_string(),
            category: ModelCategory::VLA,
            capabilities: ModalityCapabilities::vla(),
            max_context: 4096,
            version: Some("7B".to_string()),
            release_date: Some("2024-06-13".to_string()),
            description: Some("Open-source 7B parameter VLA model".to_string()),
            pricing: None,
            available: true,
            local: true,
        },
        MultimodalModel {
            id: "palm-e".to_string(),
            name: "PaLM-E".to_string(),
            provider: "Google".to_string(),
            category: ModelCategory::Embodied,
            capabilities: ModalityCapabilities::embodied(),
            max_context: 8192,
            version: Some("562B".to_string()),
            release_date: Some("2023-03-06".to_string()),
            description: Some("Embodied multimodal language model".to_string()),
            pricing: None,
            available: false,
            local: false,
        },
        MultimodalModel {
            id: "gr-1".to_string(),
            name: "GR-1".to_string(),
            provider: "Fourier Intelligence".to_string(),
            category: ModelCategory::VLA,
            capabilities: ModalityCapabilities::vla(),
            max_context: 2048,
            version: Some("1.0".to_string()),
            release_date: Some("2024-03-18".to_string()),
            description: Some("VLA for humanoid robot manipulation".to_string()),
            pricing: None,
            available: false,
            local: false,
        },
        MultimodalModel {
            id: "pi0".to_string(),
            name: "Pi-Zero".to_string(),
            provider: "Physical Intelligence".to_string(),
            category: ModelCategory::VLA,
            capabilities: ModalityCapabilities::vla(),
            max_context: 4096,
            version: Some("1.0".to_string()),
            release_date: Some("2024-10-31".to_string()),
            description: Some("General-purpose robot foundation model".to_string()),
            pricing: None,
            available: false,
            local: false,
        },
    ]
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modality_display() {
        assert_eq!(format!("{}", Modality::Text), "text");
        assert_eq!(format!("{}", Modality::Image), "image");
        assert_eq!(format!("{}", Modality::Action), "action");
    }

    #[test]
    fn test_model_category_display() {
        assert_eq!(format!("{}", ModelCategory::LLM), "LLM");
        assert_eq!(format!("{}", ModelCategory::VLM), "VLM");
        assert_eq!(format!("{}", ModelCategory::VLA), "VLA");
    }

    #[test]
    fn test_vlm_capabilities() {
        let caps = ModalityCapabilities::vlm();
        assert!(caps.supports_input(Modality::Text));
        assert!(caps.supports_input(Modality::Image));
        assert!(!caps.supports_input(Modality::Action));
        assert!(caps.supports_vision());
        assert!(!caps.supports_actions());
    }

    #[test]
    fn test_vla_capabilities() {
        let caps = ModalityCapabilities::vla();
        assert!(caps.supports_input(Modality::Text));
        assert!(caps.supports_input(Modality::Image));
        assert!(caps.supports_input(Modality::Sensor));
        assert!(caps.supports_output(Modality::Action));
        assert!(caps.supports_output(Modality::Trajectory));
        assert!(caps.supports_vision());
        assert!(caps.supports_actions());
    }

    #[test]
    fn test_multimodal_message() {
        let mut msg = MultimodalMessage::text(MessageRole::User, "What's in this image?");
        msg.add_image(ImageContent {
            data: ImageData::Url {
                url: "https://example.com/image.jpg".to_string(),
                detail: Some(ImageDetail::Auto),
            },
            format: Some(ImageFormat::Jpeg),
            alt_text: Some("Test image".to_string()),
            regions: vec![],
        });

        assert_eq!(msg.images().len(), 1);
        assert_eq!(msg.text_content(), "What's in this image?");
    }

    #[test]
    fn test_action_command() {
        let action = ActionCommand {
            action_type: ActionType::Grasp,
            parameters: ActionParameters::Gripper {
                aperture: 0.5,
                force: Some(10.0),
            },
            confidence: Some(0.95),
            timestamp: None,
            duration_ms: Some(500),
        };

        assert_eq!(action.action_type, ActionType::Grasp);
    }

    #[test]
    fn test_vlm_models_registry() {
        let models = vlm_models();
        assert!(!models.is_empty());

        let gpt4o = models.iter().find(|m| m.id == "gpt-4o").unwrap();
        assert_eq!(gpt4o.category, ModelCategory::Multimodal);
        assert!(gpt4o.available);
    }

    #[test]
    fn test_vla_models_registry() {
        let models = vla_models();
        assert!(!models.is_empty());

        let openvla = models.iter().find(|m| m.id == "openvla").unwrap();
        assert_eq!(openvla.category, ModelCategory::VLA);
        assert!(openvla.local);
    }
}
