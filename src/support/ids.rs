//! Session and conversation ids, byte-compatible with `@posthog/mcp`.

use uuid::Uuid;

pub(crate) fn new_uuid_v7() -> String {
    Uuid::now_v7().to_string()
}

pub(crate) fn new_session_id() -> String {
    format!("ses_{}", new_uuid_v7())
}

/// FNV-1a over UTF-16 code units, run twice to fill 32 hex characters. Shared
/// with the TypeScript SDK so one conversation maps to one session in both.
pub(crate) fn derive_session_id(input: &str) -> String {
    let salted = format!("{input}::salt");
    format!("ses_{}{}", fnv1a_hex(input), fnv1a_hex(&salted))
}

fn fnv1a_hex(input: &str) -> String {
    let mut low: u32 = 0x8422_2325;
    let mut high: u32 = 0xcbf2_9ce4;
    for code_unit in input.encode_utf16() {
        let code_unit = u32::from(code_unit);
        low = (low ^ code_unit).wrapping_mul(0x0000_01b3);
        high = (high ^ code_unit).wrapping_mul(0x0000_0193);
    }
    format!("{low:08x}{high:08x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_ids_are_prefixed_uuid_v7() {
        let id = new_session_id();
        let uuid = id.strip_prefix("ses_").expect("prefix");
        assert_eq!(Uuid::parse_str(uuid).unwrap().get_version_num(), 7);
    }

    #[test]
    fn derivation_matches_the_typescript_sdk() {
        let cases = [
            (
                "01a16f22-413c-7661-a1a1-829bbf0caedf",
                "ses_515209c3b6e7cc5ef65f99bb1cb88c9e",
            ),
            ("", "ses_84222325cbf29ce4805967715e2fab98"),
            (
                "h\u{e9}llo \u{1f980}",
                "ses_71e3868b77d62456fe99601365e9b0a6",
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(derive_session_id(input), expected, "input {input:?}");
        }
    }
}
