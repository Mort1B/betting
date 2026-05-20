pub(super) fn iso_to_utc_minutes(raw: &str) -> Option<i64> {
    let (date, rest) = raw.split_once('T')?;
    let zone_index = rest.find(['Z', '+', '-']).unwrap_or(rest.len());
    let time = &rest[..zone_index];
    let zone = &rest[zone_index..];
    let mut date_parts = date.split('-');
    let year = date_parts.next()?.parse::<i32>().ok()?;
    let month = date_parts.next()?.parse::<u32>().ok()?;
    let day = date_parts.next()?.parse::<u32>().ok()?;
    let mut time_parts = time.split(':');
    let hour = time_parts.next()?.parse::<i64>().ok()?;
    let minute = time_parts.next()?.parse::<i64>().ok()?;
    let offset = offset_minutes(zone)?;
    Some(days_from_civil(year, month, day) * 1440 + hour * 60 + minute - offset)
}

fn offset_minutes(zone: &str) -> Option<i64> {
    if zone.is_empty() || zone == "Z" {
        return Some(0);
    }
    let sign = match zone.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let hours = zone.get(1..3)?.parse::<i64>().ok()?;
    let minutes = zone.get(4..6)?.parse::<i64>().ok()?;
    Some(sign * (hours * 60 + minutes))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = month as i32;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i32 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    i64::from(era * 146_097 + day_of_era - 719_468)
}
