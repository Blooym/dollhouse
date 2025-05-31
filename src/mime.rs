use mime_guess::Mime;

const WILDCARD_CHAR: &str = "*";

pub fn is_mime_allowed(mime: &Mime, allowed: &Vec<Mime>) -> bool {
    if allowed.is_empty() {
        return false;
    }

    for allowed_mime in allowed {
        // MIME is '*/*', allow everything.
        if allowed_mime.type_() == WILDCARD_CHAR && allowed_mime.subtype() == WILDCARD_CHAR {
            return true;
        }

        // MIME subtype is *, allow if the type matches.
        if allowed_mime.subtype() == WILDCARD_CHAR && allowed_mime.type_() == mime.type_() {
            return true;
        }

        // Check if the mimes are exactly equal.
        if mime == allowed_mime {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use mime_guess::{Mime, mime};
    use std::str::FromStr;

    #[test]
    fn test_is_mime_allowed() {
        // Test PNG when nothing is allowed.
        assert!(
            !super::is_mime_allowed(&Mime::from_str("image/png").unwrap(), &vec![])
        );

        // Test PNG when PNG is allowed.
        assert!(
            super::is_mime_allowed(
                &Mime::from_str("image/png").unwrap(),
                &vec![mime::IMAGE_PNG],
            )
        );

        // Test PNG when only JPG is allowed.
        assert!(
            !super::is_mime_allowed(
                &Mime::from_str("image/png").unwrap(),
                &vec![mime::IMAGE_JPEG],
            )
        );

        // Test PNG when any image subtype is allowed.
        assert!(
            super::is_mime_allowed(
                &Mime::from_str("image/png").unwrap(),
                &vec![mime::IMAGE_STAR],
            )
        );

        // Test PNG when anything is allowed.
        assert!(
            super::is_mime_allowed(
                &Mime::from_str("image/png").unwrap(),
                &vec![mime::STAR_STAR],
            )
        );

        // Test HTML when any image subtype is enabled.
        assert!(
            !super::is_mime_allowed(
                &Mime::from_str("text/html").unwrap(),
                &vec![mime::IMAGE_STAR],
            )
        );

        // Test PNG when images and text are enabled.
        assert!(
            super::is_mime_allowed(
                &Mime::from_str("image/png").unwrap(),
                &vec![mime::TEXT_STAR, mime::IMAGE_STAR],
            )
        );
    }
}
