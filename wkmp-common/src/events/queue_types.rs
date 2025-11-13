//! Queue and user action type definitions
//!
//! Supporting types for queue management and user interactions.

use serde::{Deserialize, Serialize};

/// User action types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum UserActionType {
    /// User skipped current passage
    Skip,
    /// User started playback
    Play,
    /// User paused playback
    Pause,
    /// User seeked to different position
    Seek,
    /// User changed volume
    VolumeChange,
    /// User added passage to queue
    QueueAdd,
    /// User removed passage from queue
    QueueRemove,
    /// User liked current passage
    Like,
    /// User disliked current passage
    Dislike,
    /// User applied temporary override
    TemporaryOverride,
}

impl std::fmt::Display for UserActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserActionType::Skip => write!(f, "Skip"),
            UserActionType::Play => write!(f, "Play"),
            UserActionType::Pause => write!(f, "Pause"),
            UserActionType::Seek => write!(f, "Seek"),
            UserActionType::VolumeChange => write!(f, "VolumeChange"),
            UserActionType::QueueAdd => write!(f, "QueueAdd"),
            UserActionType::QueueRemove => write!(f, "QueueRemove"),
            UserActionType::Like => write!(f, "Like"),
            UserActionType::Dislike => write!(f, "Dislike"),
            UserActionType::TemporaryOverride => write!(f, "TemporaryOverride"),
        }
    }
}

/// Why the queue changed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum QueueChangeTrigger {
    /// Automatic passage selection replenished queue
    AutomaticReplenishment,
    /// User manually added passage
    UserEnqueue,
    /// User manually removed passage
    UserDequeue,
    /// Passage finished playing
    PassageCompletion,
    /// User applied temporary override
    TemporaryOverride,
}

impl std::fmt::Display for QueueChangeTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueChangeTrigger::AutomaticReplenishment => write!(f, "AutomaticReplenishment"),
            QueueChangeTrigger::UserEnqueue => write!(f, "UserEnqueue"),
            QueueChangeTrigger::UserDequeue => write!(f, "UserDequeue"),
            QueueChangeTrigger::PassageCompletion => write!(f, "PassageCompletion"),
            QueueChangeTrigger::TemporaryOverride => write!(f, "TemporaryOverride"),
        }
    }
}

/// How a passage was enqueued
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EnqueueSource {
    /// Automatically selected by program director
    Automatic,
    /// Manually added by user
    Manual,
}

impl std::fmt::Display for EnqueueSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnqueueSource::Automatic => write!(f, "Automatic"),
            EnqueueSource::Manual => write!(f, "Manual"),
        }
    }
}
