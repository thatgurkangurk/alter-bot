#[must_use]
pub fn sanitise_pings(message: &str) -> String {
    message.replace('@', "@\u{200B}")
}
