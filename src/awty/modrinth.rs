pub fn create_ferinth() -> ferinth::Ferinth<()> {
    ferinth::Ferinth::<()>::new(
        env!("CARGO_CRATE_NAME"),
        None, // version doesn't apply to this
        Some("hello@gurkz.me / https://github.com/thatgurkangurk/alter-bot"),
    )
}
