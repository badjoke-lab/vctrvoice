//! Durable stage-session markers stored in a bot-only Vector Community channel.
//! These represent SESSION bookkeeping, not proof that any audio/video is flowing.

const PREFIX: &str = "VCTRVOICE_STAGE_V1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StageStatus {
    Closed,
    Open(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Marker {
    Open(String),
    Close(String),
}

fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

fn decode(text: &str) -> Option<Marker> {
    let mut pieces = text.split_whitespace();
    if pieces.next()? != PREFIX {
        return None;
    }
    let kind = pieces.next()?;
    let id = pieces.next()?;
    if pieces.next().is_some() || !valid_id(id) {
        return None;
    }
    match kind {
        "OPEN" => Some(Marker::Open(id.to_owned())),
        "CLOSE" => Some(Marker::Close(id.to_owned())),
        _ => None,
    }
}

pub fn opened(id: &str) -> String {
    assert!(valid_id(id), "stage session ID must be 32 hex characters");
    format!("{PREFIX} OPEN {id}")
}

pub fn closed(id: &str) -> String {
    assert!(valid_id(id), "stage session ID must be 32 hex characters");
    format!("{PREFIX} CLOSE {id}")
}

pub fn replay<'a>(entries: impl IntoIterator<Item = &'a str>) -> StageStatus {
    let mut current = StageStatus::Closed;
    for entry in entries {
        match decode(entry) {
            Some(Marker::Open(id)) => current = StageStatus::Open(id),
            Some(Marker::Close(id)) => {
                if matches!(&current, StageStatus::Open(open_id) if open_id == &id) {
                    current = StageStatus::Closed;
                }
            }
            None => {}
        }
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "0123456789abcdef0123456789abcdef";
    const B: &str = "fedcba9876543210fedcba9876543210";

    #[test]
    fn opens_and_closes() {
        assert_eq!(replay([opened(A).as_str()]), StageStatus::Open(A.into()));
        assert_eq!(
            replay([opened(A).as_str(), closed(A).as_str()]),
            StageStatus::Closed
        );
    }

    #[test]
    fn ignores_unrelated_comments_and_bad_identifiers() {
        assert_eq!(replay(["Hello!", "VCTRVOICE_STAGE_V1 OPEN x", "VCTRVOICE_STAGE_V1 OPEN deadbeef tail"]), StageStatus::Closed);
        assert_eq!(
            replay(["Hello!", opened(A).as_str(), "unrelated text"]),
            StageStatus::Open(A.into())
        );
    }

    #[test]
    fn mismatched_close_does_not_end_another_session() {
        assert_eq!(
            replay([opened(A).as_str(), closed(B).as_str()]),
            StageStatus::Open(A.into())
        );
    }

    #[test]
    fn subsequent_sessions_are_separate() {
        assert_eq!(
            replay([
                opened(A).as_str(),
                closed(A).as_str(),
                opened(B).as_str(),
            ]),
            StageStatus::Open(B.into())
        );
    }

    #[test]
    fn rejects_noncanonical_markers() {
        assert_eq!(
            replay(["VCTRVOICE_STAGE_V2 OPEN 0123456789abcdef0123456789abcdef",
                    "VCTRVOICE_STAGE_V1 OPEN 0123456789abcdef0123456789abcdef extra"]),
            StageStatus::Closed
        );
    }
}
