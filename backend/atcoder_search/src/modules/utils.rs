pub fn rate_to_color(rate: i32) -> String {
    match rate {
        0..=399 => "gray",
        400..=799 => "brown",
        800..=1199 => "green",
        1200..=1599 => "cyan",
        1600..=1999 => "blue",
        2000..=2399 => "yellow",
        2400..=2799 => "orange",
        2800..=3199 => "red",
        3200..=3599 => "silver",
        _ => "gold",
    }
    .to_string()
}
