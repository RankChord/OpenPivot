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
host = "localhost"
port = "5432"
username = "openpivot"
password = "openpivot"
database = "openpivot"
max_connections = 10
"#;