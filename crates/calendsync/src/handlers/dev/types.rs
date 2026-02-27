//! AFS-aligned domain types and pure business logic for dev annotations.

#[cfg(feature = "dev-annotations")]
use std::fmt;

#[cfg(feature = "dev-annotations")]
use serde::{Deserialize, Serialize};

// ============================================================================
// Enums
// ============================================================================

/// What the annotator wants to happen.
#[cfg(feature = "dev-annotations")]
#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationIntent {
    Fix,
    Change,
    Question,
    Approve,
}

/// How urgent the annotation is.
#[cfg(feature = "dev-annotations")]
#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationSeverity {
    Blocking,
    Important,
    Suggestion,
}

/// Lifecycle status of an annotation.
#[cfg(feature = "dev-annotations")]
#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationStatus {
    Pending,
    Acknowledged,
    Resolved,
    Dismissed,
}

/// Lifecycle status of a dev session.
#[cfg(feature = "dev-annotations")]
#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Closed,
}

/// Who authored a thread message.
#[cfg(feature = "dev-annotations")]
#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThreadAuthor {
    Human,
    Agent,
}

// ============================================================================
// Structs
// ============================================================================

/// A message within an annotation's discussion thread.
#[cfg(feature = "dev-annotations")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ThreadMessage {
    pub id: String,
    pub annotation_id: String,
    pub message: String,
    pub author: ThreadAuthor,
    pub timestamp: String,
}

/// A dev review session grouping multiple annotations.
#[cfg(feature = "dev-annotations")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevSession {
    pub id: String,
    pub url: String,
    pub status: SessionStatus,
    pub created_at: String,
    pub updated_at: String,
}

/// ARIA / accessibility metadata captured from the target element.
#[cfg(feature = "dev-annotations")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccessibilityInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Bounding rectangle of the annotated element.
#[cfg(feature = "dev-annotations")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BoundingBox {
    pub top: f64,
    pub left: f64,
    pub width: f64,
    pub height: f64,
}

/// Computed CSS styles of the annotated element.
#[cfg(feature = "dev-annotations")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComputedStyles {
    pub color: String,
    pub background_color: String,
    pub font_size: String,
    pub font_family: String,
    pub padding: String,
    pub margin: String,
    pub width: String,
    pub height: String,
    pub display: String,
    pub position: String,
}

// ============================================================================
// DevAnnotation
// ============================================================================

/// A UI annotation created in dev mode for human-agent collaboration.
///
/// Captures element metadata, computed styles, discussion thread, and
/// lifecycle status so both the developer and Claude Code can track
/// feedback through resolution.
#[cfg(feature = "dev-annotations")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevAnnotation {
    // --- Retained fields ---
    pub id: String,
    pub timestamp: String,
    pub tag_name: String,
    #[serde(default)]
    pub text_content: String,
    pub bounding_box: BoundingBox,
    pub computed_styles: ComputedStyles,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component_name: Option<String>,

    // --- AFS fields ---
    pub session_id: String,
    pub url: String,
    pub element_path: String,
    pub comment: String,
    pub intent: AnnotationIntent,
    pub severity: AnnotationSeverity,
    pub status: AnnotationStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nearby_text: Option<String>,
    #[serde(default)]
    pub css_classes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_path: Option<String>,
    #[serde(default)]
    pub is_fixed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_by: Option<String>,
    #[serde(default)]
    pub thread: Vec<ThreadMessage>,
}

// ============================================================================
// Broadcast events
// ============================================================================

/// Server-sent events for real-time annotation updates.
#[cfg(feature = "dev-annotations")]
#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DevAnnotationEvent {
    Created(DevAnnotation),
    Updated(DevAnnotation),
    Deleted { id: String },
    ThreadMessage(ThreadMessage),
}

// ============================================================================
// Status transition validation
// ============================================================================

/// Error returned when a status transition is not allowed.
#[cfg(feature = "dev-annotations")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusTransitionError {
    /// The annotation is already in the target status.
    SameStatus,
    /// The transition from `from` to `to` is not permitted.
    InvalidTransition {
        from: AnnotationStatus,
        to: AnnotationStatus,
    },
}

#[cfg(feature = "dev-annotations")]
impl fmt::Display for StatusTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StatusTransitionError::SameStatus => {
                write!(f, "annotation is already in the target status")
            }
            StatusTransitionError::InvalidTransition { from, to } => {
                write!(f, "cannot transition from {from:?} to {to:?}")
            }
        }
    }
}

#[cfg(feature = "dev-annotations")]
impl std::error::Error for StatusTransitionError {}

/// Validates whether transitioning from `current` to `target` is allowed.
///
/// Valid transitions:
/// - Pending -> Acknowledged, Resolved, Dismissed
/// - Acknowledged -> Resolved, Dismissed
///
/// Resolved and Dismissed are terminal states.
#[cfg(feature = "dev-annotations")]
pub fn validate_status_transition(
    current: AnnotationStatus,
    target: AnnotationStatus,
) -> Result<(), StatusTransitionError> {
    if current == target {
        return Err(StatusTransitionError::SameStatus);
    }

    let valid = matches!(
        (current, target),
        (AnnotationStatus::Pending, AnnotationStatus::Acknowledged)
            | (AnnotationStatus::Pending, AnnotationStatus::Resolved)
            | (AnnotationStatus::Pending, AnnotationStatus::Dismissed)
            | (AnnotationStatus::Acknowledged, AnnotationStatus::Resolved)
            | (AnnotationStatus::Acknowledged, AnnotationStatus::Dismissed)
    );

    if valid {
        Ok(())
    } else {
        Err(StatusTransitionError::InvalidTransition {
            from: current,
            to: target,
        })
    }
}

// ============================================================================
// Pure query helpers
// ============================================================================

/// Breakdown of annotation counts by status.
#[cfg(feature = "dev-annotations")]
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct AnnotationSummary {
    pub total: usize,
    pub pending: usize,
    pub acknowledged: usize,
    pub resolved: usize,
    pub dismissed: usize,
}

/// Counts annotations grouped by status.
#[cfg(feature = "dev-annotations")]
pub fn count_annotations_summary(annotations: &[DevAnnotation]) -> AnnotationSummary {
    let mut pending = 0;
    let mut acknowledged = 0;
    let mut resolved = 0;
    let mut dismissed = 0;

    for annotation in annotations {
        match annotation.status {
            AnnotationStatus::Pending => pending += 1,
            AnnotationStatus::Acknowledged => acknowledged += 1,
            AnnotationStatus::Resolved => resolved += 1,
            AnnotationStatus::Dismissed => dismissed += 1,
        }
    }

    AnnotationSummary {
        total: annotations.len(),
        pending,
        acknowledged,
        resolved,
        dismissed,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(all(test, feature = "dev-annotations"))]
mod tests {
    use super::*;

    // -- Test helpers --------------------------------------------------------

    fn sample_bounding_box() -> BoundingBox {
        BoundingBox {
            top: 100.0,
            left: 200.0,
            width: 300.0,
            height: 50.0,
        }
    }

    fn sample_computed_styles() -> ComputedStyles {
        ComputedStyles {
            color: "rgb(0, 0, 0)".to_string(),
            background_color: "rgb(255, 255, 255)".to_string(),
            font_size: "16px".to_string(),
            font_family: "Inter, sans-serif".to_string(),
            padding: "8px".to_string(),
            margin: "0px".to_string(),
            width: "300px".to_string(),
            height: "50px".to_string(),
            display: "block".to_string(),
            position: "relative".to_string(),
        }
    }

    fn sample_annotation(
        id: &str,
        status: AnnotationStatus,
        intent: AnnotationIntent,
        severity: AnnotationSeverity,
    ) -> DevAnnotation {
        DevAnnotation {
            id: id.to_string(),
            timestamp: "2024-01-15T10:00:00Z".to_string(),
            tag_name: "h1".to_string(),
            text_content: "January 2024".to_string(),
            bounding_box: sample_bounding_box(),
            computed_styles: sample_computed_styles(),
            screenshot: None,
            component_name: Some("CalendarHeader".to_string()),
            session_id: "session-1".to_string(),
            url: "http://localhost:3000/calendar/abc".to_string(),
            element_path: "div.calendar > h1".to_string(),
            comment: "Font size too small on mobile".to_string(),
            intent,
            severity,
            status,
            selected_text: None,
            nearby_text: None,
            css_classes: vec!["calendar-header".to_string()],
            accessibility: None,
            full_path: None,
            is_fixed: false,
            resolved_at: None,
            resolved_by: None,
            thread: vec![],
        }
    }

    // -- Status transition tests ---------------------------------------------

    #[test]
    fn valid_transition_pending_to_acknowledged() {
        assert!(validate_status_transition(
            AnnotationStatus::Pending,
            AnnotationStatus::Acknowledged
        )
        .is_ok());
    }

    #[test]
    fn valid_transition_pending_to_resolved() {
        assert!(
            validate_status_transition(AnnotationStatus::Pending, AnnotationStatus::Resolved)
                .is_ok()
        );
    }

    #[test]
    fn valid_transition_pending_to_dismissed() {
        assert!(
            validate_status_transition(AnnotationStatus::Pending, AnnotationStatus::Dismissed)
                .is_ok()
        );
    }

    #[test]
    fn valid_transition_acknowledged_to_resolved() {
        assert!(validate_status_transition(
            AnnotationStatus::Acknowledged,
            AnnotationStatus::Resolved
        )
        .is_ok());
    }

    #[test]
    fn valid_transition_acknowledged_to_dismissed() {
        assert!(validate_status_transition(
            AnnotationStatus::Acknowledged,
            AnnotationStatus::Dismissed
        )
        .is_ok());
    }

    #[test]
    fn invalid_transition_same_status_pending() {
        assert_eq!(
            validate_status_transition(AnnotationStatus::Pending, AnnotationStatus::Pending),
            Err(StatusTransitionError::SameStatus)
        );
    }

    #[test]
    fn invalid_transition_same_status_resolved() {
        assert_eq!(
            validate_status_transition(AnnotationStatus::Resolved, AnnotationStatus::Resolved),
            Err(StatusTransitionError::SameStatus)
        );
    }

    #[test]
    fn invalid_transition_same_status_dismissed() {
        assert_eq!(
            validate_status_transition(AnnotationStatus::Dismissed, AnnotationStatus::Dismissed),
            Err(StatusTransitionError::SameStatus)
        );
    }

    #[test]
    fn invalid_transition_same_status_acknowledged() {
        assert_eq!(
            validate_status_transition(
                AnnotationStatus::Acknowledged,
                AnnotationStatus::Acknowledged
            ),
            Err(StatusTransitionError::SameStatus)
        );
    }

    #[test]
    fn invalid_transition_resolved_to_pending() {
        assert_eq!(
            validate_status_transition(AnnotationStatus::Resolved, AnnotationStatus::Pending),
            Err(StatusTransitionError::InvalidTransition {
                from: AnnotationStatus::Resolved,
                to: AnnotationStatus::Pending,
            })
        );
    }

    #[test]
    fn invalid_transition_resolved_to_acknowledged() {
        assert_eq!(
            validate_status_transition(AnnotationStatus::Resolved, AnnotationStatus::Acknowledged),
            Err(StatusTransitionError::InvalidTransition {
                from: AnnotationStatus::Resolved,
                to: AnnotationStatus::Acknowledged,
            })
        );
    }

    #[test]
    fn invalid_transition_dismissed_to_pending() {
        assert_eq!(
            validate_status_transition(AnnotationStatus::Dismissed, AnnotationStatus::Pending),
            Err(StatusTransitionError::InvalidTransition {
                from: AnnotationStatus::Dismissed,
                to: AnnotationStatus::Pending,
            })
        );
    }

    #[test]
    fn invalid_transition_acknowledged_to_pending() {
        assert_eq!(
            validate_status_transition(AnnotationStatus::Acknowledged, AnnotationStatus::Pending),
            Err(StatusTransitionError::InvalidTransition {
                from: AnnotationStatus::Acknowledged,
                to: AnnotationStatus::Pending,
            })
        );
    }

    #[test]
    fn invalid_transition_resolved_to_dismissed() {
        assert_eq!(
            validate_status_transition(AnnotationStatus::Resolved, AnnotationStatus::Dismissed),
            Err(StatusTransitionError::InvalidTransition {
                from: AnnotationStatus::Resolved,
                to: AnnotationStatus::Dismissed,
            })
        );
    }

    #[test]
    fn invalid_transition_dismissed_to_resolved() {
        assert_eq!(
            validate_status_transition(AnnotationStatus::Dismissed, AnnotationStatus::Resolved),
            Err(StatusTransitionError::InvalidTransition {
                from: AnnotationStatus::Dismissed,
                to: AnnotationStatus::Resolved,
            })
        );
    }

    // -- count_annotations_summary tests -------------------------------------

    #[test]
    fn summary_empty() {
        let annotations: Vec<DevAnnotation> = vec![];
        assert_eq!(
            count_annotations_summary(&annotations),
            AnnotationSummary {
                total: 0,
                pending: 0,
                acknowledged: 0,
                resolved: 0,
                dismissed: 0,
            }
        );
    }

    #[test]
    fn summary_all_pending() {
        let annotations = vec![
            sample_annotation(
                "a",
                AnnotationStatus::Pending,
                AnnotationIntent::Fix,
                AnnotationSeverity::Important,
            ),
            sample_annotation(
                "b",
                AnnotationStatus::Pending,
                AnnotationIntent::Change,
                AnnotationSeverity::Suggestion,
            ),
        ];
        assert_eq!(
            count_annotations_summary(&annotations),
            AnnotationSummary {
                total: 2,
                pending: 2,
                acknowledged: 0,
                resolved: 0,
                dismissed: 0,
            }
        );
    }

    #[test]
    fn summary_mixed_statuses() {
        let annotations = vec![
            sample_annotation(
                "a",
                AnnotationStatus::Pending,
                AnnotationIntent::Fix,
                AnnotationSeverity::Blocking,
            ),
            sample_annotation(
                "b",
                AnnotationStatus::Acknowledged,
                AnnotationIntent::Change,
                AnnotationSeverity::Important,
            ),
            sample_annotation(
                "c",
                AnnotationStatus::Resolved,
                AnnotationIntent::Question,
                AnnotationSeverity::Suggestion,
            ),
            sample_annotation(
                "d",
                AnnotationStatus::Dismissed,
                AnnotationIntent::Approve,
                AnnotationSeverity::Suggestion,
            ),
            sample_annotation(
                "e",
                AnnotationStatus::Pending,
                AnnotationIntent::Fix,
                AnnotationSeverity::Blocking,
            ),
        ];
        assert_eq!(
            count_annotations_summary(&annotations),
            AnnotationSummary {
                total: 5,
                pending: 2,
                acknowledged: 1,
                resolved: 1,
                dismissed: 1,
            }
        );
    }

    // -- Serde roundtrip tests -----------------------------------------------

    #[test]
    fn dev_annotation_serde_roundtrip() {
        let annotation = sample_annotation(
            "test-id",
            AnnotationStatus::Pending,
            AnnotationIntent::Fix,
            AnnotationSeverity::Blocking,
        );
        let json = serde_json::to_string(&annotation).unwrap();
        let deserialized: DevAnnotation = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "test-id");
        assert_eq!(deserialized.comment, "Font size too small on mobile");
        assert_eq!(deserialized.status, AnnotationStatus::Pending);
        assert_eq!(deserialized.intent, AnnotationIntent::Fix);
        assert_eq!(deserialized.severity, AnnotationSeverity::Blocking);
        assert_eq!(deserialized.session_id, "session-1");
        assert!(deserialized.thread.is_empty());
    }

    #[test]
    fn dev_annotation_serde_skips_none_optionals() {
        let annotation = sample_annotation(
            "skip-test",
            AnnotationStatus::Pending,
            AnnotationIntent::Question,
            AnnotationSeverity::Suggestion,
        );
        let json = serde_json::to_string(&annotation).unwrap();

        // Fields with skip_serializing_if = "Option::is_none" should be absent
        assert!(!json.contains("\"screenshot\""));
        assert!(!json.contains("\"selected_text\""));
        assert!(!json.contains("\"nearby_text\""));
        assert!(!json.contains("\"accessibility\""));
        assert!(!json.contains("\"full_path\""));
        assert!(!json.contains("\"resolved_at\""));
        assert!(!json.contains("\"resolved_by\""));
    }

    #[test]
    fn dev_annotation_serde_includes_present_optionals() {
        let mut annotation = sample_annotation(
            "opt-test",
            AnnotationStatus::Resolved,
            AnnotationIntent::Fix,
            AnnotationSeverity::Blocking,
        );
        annotation.resolved_at = Some("2024-01-16T12:00:00Z".to_string());
        annotation.resolved_by = Some("agent".to_string());
        annotation.accessibility = Some(AccessibilityInfo {
            role: Some("heading".to_string()),
            label: None,
            description: None,
        });

        let json = serde_json::to_string(&annotation).unwrap();
        let deserialized: DevAnnotation = serde_json::from_str(&json).unwrap();

        assert_eq!(
            deserialized.resolved_at,
            Some("2024-01-16T12:00:00Z".to_string())
        );
        assert_eq!(deserialized.resolved_by, Some("agent".to_string()));

        let a11y = deserialized.accessibility.unwrap();
        assert_eq!(a11y.role, Some("heading".to_string()));
        assert!(a11y.label.is_none());
        assert!(a11y.description.is_none());
    }

    #[test]
    fn dev_annotation_event_serializes_with_tag() {
        let annotation = sample_annotation(
            "evt-test",
            AnnotationStatus::Pending,
            AnnotationIntent::Fix,
            AnnotationSeverity::Blocking,
        );
        let event = DevAnnotationEvent::Created(annotation);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"created\""));

        let deleted = DevAnnotationEvent::Deleted {
            id: "del-1".to_string(),
        };
        let json = serde_json::to_string(&deleted).unwrap();
        assert!(json.contains("\"type\":\"deleted\""));
        assert!(json.contains("\"id\":\"del-1\""));
    }

    #[test]
    fn status_transition_error_display() {
        let same = StatusTransitionError::SameStatus;
        assert_eq!(
            same.to_string(),
            "annotation is already in the target status"
        );

        let invalid = StatusTransitionError::InvalidTransition {
            from: AnnotationStatus::Resolved,
            to: AnnotationStatus::Pending,
        };
        assert_eq!(
            invalid.to_string(),
            "cannot transition from Resolved to Pending"
        );
    }
}
