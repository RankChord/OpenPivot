use crate::job::Schedule;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert!(Schedule::parse("30m").is_ok());
        assert!(Schedule::parse("2h").is_ok());
        assert!(Schedule::parse("1d").is_ok());
        assert!(Schedule::parse("60s").is_ok());
    }

    #[test]
    fn test_parse_every() {
        let s = Schedule::parse("every 2h").unwrap();
        assert!(matches!(s, Schedule::Every(_)));
    }

    #[test]
    fn test_parse_cron() {
        let s = Schedule::parse("0 9 * * *").unwrap();
        assert!(matches!(s, Schedule::Cron(_)));
    }

    #[test]
    fn test_parse_iso_timestamp() {
        let s = Schedule::parse("2026-06-01T09:00:00Z").unwrap();
        assert!(matches!(s, Schedule::Once(_)));
    }

    #[test]
    fn test_parse_invalid() {
        assert!(Schedule::parse("not a schedule").is_err());
    }
}
