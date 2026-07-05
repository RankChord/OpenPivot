pub const DEFAULT_CONFIG: &str = r#"
[server]
host = "127.0.0.1"
port = 3000

[app]
enable = false
debug = true
domain = "rankchord.com"
federal = false

[database]
host = "10.0.0.11"
port = 5432
username = "openpivot"
password = "openpivot"
database = "openpivot"
max_connections = 10

[minio]
host = "10.0.0.11"
port = 9000
username = "openpivot"
password = "openpivot"
bucket = "openpivot"

[auth]
jwt_secret = "EPo1Rb3y027FqOVELx+SwPC6GEASYWcCKXuhaQWxnFiOH6zWF6YactE1jq68wF5d12KhkOo0bswhFhEqYCRCBQ=="
access_token_ttl_minutes = 15
refresh_token_ttl_days = 7
"#;